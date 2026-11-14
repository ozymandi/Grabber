//! MCP server for LLM clients (OpenSpec change add-mcp-editing, design D2-D7). The
//! `ripper-mcp` binary feeds it newline-delimited JSON-RPC from stdin; everything here is
//! plain functions on the workspace store so it can be tested without a process.
//!
//! The server never edits pictures itself: it hands the client PNG paths and thumbnails, takes
//! back PNG paths as new tile pictures, and writes layout changes only as proposals that the
//! ATLAS window shows with APPLY / DISCARD.

use crate::atlas::edit::{self, Edit};
use crate::atlas::files;
use crate::atlas::history::{self, PictureWrite};
use crate::atlas::model::{Filter, Manifest, Sector};
use crate::atlas::store::{Store, Workspace};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const PROTOCOL: &str = "2025-06-18";
/// A window's published selection older than this means no window (design D5).
pub const UI_STALE_MS: u64 = 5000;
/// Longest side of thumbnails in tool results.
pub const THUMB: u32 = 256;
/// File the ATLAS window writes its selection to, in the workspace folder.
pub const UI_STATE: &str = "ui-state.json";
/// File a proposal is written to, in the workspace folder.
pub const PROPOSAL: &str = "proposal.json";

const INSTRUCTIONS: &str = "ripper is a texture atlas tool. You edit tile pictures with your own means \
(your image generation, skills such as ComfyUI, other programs); ripper only gives you PNG paths and takes \
new pictures back. Working rules: \
1) Call get_scope first. It follows the user's selection in the ripper window. If it returns confirm=true \
(the whole atlas), ask the user before writing anything. \
2) Before write_tile or write_sector, show the user your plan and thumbnails and wait for their yes. \
3) You cannot move, scale, rotate or delete tiles, change sectors or export. To suggest such changes, call \
propose; the user sees a preview in ripper and applies or discards it. \
4) Pass scope_id from get_scope to write_tile so sector-wide edits are recorded as one history entry. \
5) Every write keeps the earlier pictures in the tile's history; go_to_state brings one back.";

/// A selection published by the ATLAS window (design D5).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct UiState {
    #[serde(default)]
    pub tiles: Vec<u64>,
    #[serde(default)]
    pub sectors: Vec<Sector>,
    /// Milliseconds since the Unix epoch of the last write.
    #[serde(default)]
    pub at: u64,
}

/// A layout proposal (design D7).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Proposal {
    pub text: String,
    pub by: String,
    pub at: String,
    pub base_revision: u64,
    pub edits: Vec<Edit>,
}

struct FrozenScope {
    workspace: String,
    tiles: Vec<u64>,
    source: &'static str,
}

pub struct Server {
    pub store: Store,
    pub config: PathBuf,
    pub temp: PathBuf,
    client: String,
    scopes: HashMap<String, FrozenScope>,
    next_scope: u64,
}

pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// Readable author name from the client's `clientInfo.name`.
fn author(name: &str) -> String {
    let n = name.to_ascii_lowercase();
    if n.contains("claude") {
        "Claude Code".into()
    } else if n.contains("codex") {
        "Codex".into()
    } else if n.contains("gemini") {
        "Gemini CLI".into()
    } else if name.trim().is_empty() {
        "LLM".into()
    } else {
        name.trim().into()
    }
}

/// UDIM label of a sector (as ATLAS shows it), and back.
pub fn label(m: &Manifest, s: Sector) -> u32 {
    history::sector_label(m, s)
}

pub fn sector_by_label(m: &Manifest, label: u64) -> Result<Sector, String> {
    let u0 = m.sectors.iter().map(|c| c.u).min().unwrap_or(0);
    let v0 = m.sectors.iter().map(|c| c.v).min().unwrap_or(0);
    if label < 1001 {
        return Err(format!("{label} is not a sector number"));
    }
    let k = (label - 1001) as i32;
    let s = Sector { u: u0 + k % 10, v: v0 + k / 10 };
    if m.has_sector(s) {
        Ok(s)
    } else {
        Err(format!("There is no sector {label}"))
    }
}

/// Minimal standard base64 for image content.
pub fn base64(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        out.push(A[(n >> 18) as usize & 63] as char);
        out.push(A[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { A[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { A[n as usize & 63] as char } else { '=' });
    }
    out
}

fn text(v: Value) -> Value {
    json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&v).unwrap() }], "structuredContent": v })
}

fn tool_error(msg: impl Into<String>) -> Value {
    json!({ "content": [{ "type": "text", "text": msg.into() }], "isError": true })
}

fn arg_u64(a: &Value, k: &str) -> Result<u64, String> {
    a.get(k).and_then(Value::as_u64).ok_or_else(|| format!("'{k}' (a number) is required"))
}

fn arg_str<'a>(a: &'a Value, k: &str) -> Result<&'a str, String> {
    a.get(k).and_then(Value::as_str).filter(|s| !s.trim().is_empty()).ok_or_else(|| format!("'{k}' (text) is required"))
}

fn arg_filter(a: &Value) -> Result<Filter, String> {
    match a.get("filter").and_then(Value::as_str).unwrap_or("smooth") {
        "smooth" => Ok(Filter::Smooth),
        "nearest" => Ok(Filter::Nearest),
        f => Err(format!("filter '{f}' is not 'smooth' or 'nearest'")),
    }
}

fn read_png(path: &str) -> Result<crate::atlas::render::Image, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    files::decode_png(&bytes).map_err(|e| format!("{path} is not a readable image: {e}"))
}

impl Server {
    pub fn new(store: Store, config: PathBuf, temp: PathBuf) -> Server {
        clean_old(&temp);
        Server { store, config, temp, client: "LLM".into(), scopes: HashMap::new(), next_scope: 1 }
    }

    /// The server for this machine: `%APPDATA%\ripper`, or `RIPPER_MCP_HOME` (tests).
    pub fn for_machine() -> Server {
        let (root, config, temp) = match std::env::var_os("RIPPER_MCP_HOME") {
            Some(h) => {
                let h = PathBuf::from(h);
                (h.join("workspaces"), h.join("config.toml"), h.join("temp"))
            }
            None => (crate::atlas::store::default_root(), crate::settings::default_path(), std::env::temp_dir().join("ripper-mcp")),
        };
        Server::new(Store::new(root), config, temp)
    }

    /// Handles one JSON-RPC message; `None` for notifications.
    pub fn handle(&mut self, msg: &Value) -> Option<Value> {
        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(json!({}));
        let id = id?;
        let result = match method {
            "initialize" => {
                if let Some(n) = params.pointer("/clientInfo/name").and_then(Value::as_str) {
                    self.client = author(n);
                }
                let version = params.get("protocolVersion").and_then(Value::as_str).unwrap_or(PROTOCOL);
                Ok(json!({
                    "protocolVersion": version,
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "ripper", "version": env!("CARGO_PKG_VERSION") },
                    "instructions": INSTRUCTIONS,
                }))
            }
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tools() })),
            "tools/call" => {
                let name = params.get("name").and_then(Value::as_str).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                Ok(match self.call(name, &args) {
                    Ok(v) => v,
                    Err(e) => tool_error(e),
                })
            }
            _ => Err((-32601, format!("Method not found: {method}"))),
        };
        Some(match result {
            Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
            Err((code, m)) => json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": m } }),
        })
    }

    fn workspace(&self) -> Result<Workspace, String> {
        let (s, _) = crate::settings::load(&self.config);
        if s.current_workspace.is_empty() {
            return Err("ripper has no current workspace yet: open ripper, switch to ATLAS and start one".into());
        }
        self.store.existing(&s.current_workspace).ok_or_else(|| format!("the current workspace {} is missing", s.current_workspace))
    }

    fn temp_dir(&self, ws: &Workspace) -> Result<PathBuf, String> {
        let d = self.temp.join(&ws.id);
        std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
        Ok(d)
    }

    /// The window's selection, if a window published it within [`UI_STALE_MS`].
    pub fn ui_state(ws: &Workspace) -> Option<UiState> {
        let text = std::fs::read_to_string(ws.dir.join(UI_STATE)).ok()?;
        let s: UiState = serde_json::from_str(&text).ok()?;
        (now_ms().saturating_sub(s.at) <= UI_STALE_MS).then_some(s)
    }

    fn call(&mut self, name: &str, a: &Value) -> Result<Value, String> {
        match name {
            "atlas_info" => self.atlas_info(),
            "get_scope" => self.get_scope(a),
            "get_tile" => self.get_tile(a),
            "render_sector" => self.render_sector(a),
            "write_tile" => self.write_tile(a),
            "write_sector" => self.write_sector(a),
            "go_to_state" => self.go_to_state(a),
            "propose" => self.propose(a),
            _ => Err(format!("unknown tool {name}")),
        }
    }

    fn tile_json(m: &Manifest, t: &crate::atlas::model::Tile) -> Value {
        let r = t.rect();
        json!({
            "id": t.id,
            "sector": m.sector_of(t.id).map(|s| label(m, s)),
            "rect": { "x": r.x, "y": r.y, "w": r.w, "h": r.h },
            "quarter_turns": t.frame.rot,
            "tilt": t.frame.tilt,
            "picture": [t.w0, t.h0],
            "source": format!("{} Q{}", t.source.stem, t.source.quad),
            "states": t.history.len().max(1),
            "current": t.current,
        })
    }

    fn atlas_info(&self) -> Result<Value, String> {
        let ws = self.workspace()?;
        let m = ws.read().map_err(|e| e.to_string())?;
        let sectors: Vec<Value> = m.sectors.iter().map(|s| json!({ "label": label(&m, *s), "u": s.u, "v": s.v })).collect();
        let log: Vec<Value> = m
            .sector_log
            .iter()
            .enumerate()
            .map(|(i, e)| json!({ "index": i, "sector": label(&m, e.sector), "name": e.name, "by": e.by, "at": e.at, "tiles": e.tiles.iter().map(|c| c.id).collect::<Vec<_>>() }))
            .collect();
        Ok(text(json!({
            "workspace": { "id": ws.id, "name": m.name },
            "sector_size": m.sector_size,
            "sectors": sectors,
            "active_sector": m.active.map(|s| label(&m, s)),
            "window_open": Self::ui_state(&ws).is_some(),
            "tiles": m.tiles.iter().map(|t| Self::tile_json(&m, t)).collect::<Vec<_>>(),
            "sector_history": log,
        })))
    }

    /// Resolves the scope (spec atlas-mcp "Scope follows the selection") and freezes it.
    fn get_scope(&mut self, a: &Value) -> Result<Value, String> {
        let ws = self.workspace()?;
        let m = ws.read().map_err(|e| e.to_string())?;
        let ui = Self::ui_state(&ws);
        let in_sectors = |secs: &[Sector]| -> Vec<u64> { m.tiles.iter().filter(|t| m.sector_of(t.id).is_some_and(|s| secs.contains(&s))).map(|t| t.id).collect() };
        let target = a.get("target");
        let (source, tiles, sectors): (&'static str, Vec<u64>, Vec<Sector>) = if let Some(t) = target.and_then(|t| t.get("tiles")).and_then(Value::as_array) {
            let ids: Vec<u64> = t.iter().filter_map(Value::as_u64).collect();
            for id in &ids {
                if m.tile(*id).is_none() {
                    return Err(format!("there is no tile {id}"));
                }
            }
            ("request", ids, Vec::new())
        } else if let Some(s) = target.and_then(|t| t.get("sectors")).and_then(Value::as_array) {
            let secs = s.iter().filter_map(Value::as_u64).map(|l| sector_by_label(&m, l)).collect::<Result<Vec<_>, _>>()?;
            ("request_sectors", in_sectors(&secs), secs)
        } else if target.and_then(|t| t.get("all")).and_then(Value::as_bool) == Some(true) {
            ("request_atlas", m.tiles.iter().map(|t| t.id).collect(), m.sectors.clone())
        } else if let Some(u) = ui.as_ref().filter(|u| !u.tiles.is_empty()) {
            ("tiles", u.tiles.iter().copied().filter(|id| m.tile(*id).is_some()).collect(), Vec::new())
        } else if let Some(u) = ui.as_ref().filter(|u| !u.sectors.is_empty()) {
            ("sectors", in_sectors(&u.sectors), u.sectors.clone())
        } else {
            ("atlas", m.tiles.iter().map(|t| t.id).collect(), m.sectors.clone())
        };
        let confirm = source == "atlas";
        let id = format!("s{}", self.next_scope);
        self.next_scope += 1;
        self.scopes.insert(id.clone(), FrozenScope { workspace: ws.id.clone(), tiles: tiles.clone(), source });
        let note = if confirm {
            format!("Nothing is selected in ripper{}: the scope is the whole atlas ({} tiles). Ask the user before writing.", if ui.is_some() { "" } else { " (no window open)" }, tiles.len())
        } else {
            format!("{} tiles", tiles.len())
        };
        Ok(text(json!({
            "scope_id": id,
            "source": source,
            "window_open": ui.is_some(),
            "confirm": confirm,
            "sectors": sectors.iter().map(|s| label(&m, *s)).collect::<Vec<_>>(),
            "tiles": tiles.iter().filter_map(|id| m.tile(*id)).map(|t| Self::tile_json(&m, t)).collect::<Vec<_>>(),
            "note": note,
        })))
    }

    fn get_tile(&self, a: &Value) -> Result<Value, String> {
        let ws = self.workspace()?;
        let m = ws.read().map_err(|e| e.to_string())?;
        let id = arg_u64(a, "id")?;
        let t = m.tile(id).ok_or_else(|| format!("there is no tile {id}"))?;
        let path = self.temp_dir(&ws)?.join(format!("tile-{id}-state{}.png", t.current));
        std::fs::copy(ws.item_path(&t.item), &path).map_err(|e| e.to_string())?;
        let thumb = files::thumbnail(&ws, &t.item, THUMB).map_err(|e| e.to_string())?;
        let states: Vec<Value> = t
            .history
            .iter()
            .enumerate()
            .map(|(i, s)| json!({ "index": i, "name": s.name, "by": s.by, "at": s.at, "size": [s.w0, s.h0], "filter": s.filter, "current": i == t.current }))
            .collect();
        let info = json!({ "tile": Self::tile_json(&m, t), "path": path.display().to_string(), "states": states });
        let mut r = text(info);
        r["content"].as_array_mut().unwrap().push(json!({ "type": "image", "data": base64(&thumb), "mimeType": "image/png" }));
        Ok(r)
    }

    fn render_sector(&self, a: &Value) -> Result<Value, String> {
        let ws = self.workspace()?;
        let m = ws.read().map_err(|e| e.to_string())?;
        let l = arg_u64(a, "sector")?;
        let s = sector_by_label(&m, l)?;
        let (img, tiles) = ws.render_sector(s).map_err(|e| e.to_string())?;
        let path = self.temp_dir(&ws)?.join(format!("sector-{l}.png"));
        std::fs::write(&path, files::encode_png(&img)).map_err(|e| e.to_string())?;
        Ok(text(json!({ "sector": l, "path": path.display().to_string(), "size": [img.w, img.h], "tiles": tiles })))
    }

    fn write_tile(&mut self, a: &Value) -> Result<Value, String> {
        let ws = self.workspace()?;
        let name = arg_str(a, "name")?;
        let keep = a.get("keep_alpha").and_then(Value::as_bool).unwrap_or(true);
        let filter = arg_filter(a)?;
        let list = a.get("tiles").and_then(Value::as_array).ok_or("'tiles' (a list of {id, path}) is required")?;
        let scope = match a.get("scope_id").and_then(Value::as_str) {
            Some(sid) => Some(self.scopes.get(sid).ok_or_else(|| format!("unknown scope_id {sid}; call get_scope again"))?),
            None => None,
        };
        if let Some(s) = scope {
            if s.workspace != ws.id {
                return Err("the scope belongs to another workspace; call get_scope again".into());
            }
        }
        let mut writes = Vec::new();
        let mut early: Vec<Value> = Vec::new();
        for e in list {
            let id = arg_u64(e, "id")?;
            if scope.is_some_and(|s| !s.tiles.contains(&id)) {
                early.push(json!({ "id": id, "reason": "not in the scope" }));
                continue;
            }
            match read_png(arg_str(e, "path")?) {
                Ok(image) => writes.push(PictureWrite { id, image, filter }),
                Err(r) => early.push(json!({ "id": id, "reason": r })),
            }
        }
        let sector_entries = scope.is_some_and(|s| matches!(s.source, "sectors" | "atlas" | "request_sectors" | "request_atlas"));
        let (m, rep) = ws.write_pictures(writes, name, &self.client, keep, sector_entries).map_err(|e| e.to_string())?;
        let mut skipped: Vec<Value> = rep.skipped.iter().map(|(id, r)| json!({ "id": id, "reason": r })).collect();
        skipped.extend(early);
        Ok(text(json!({
            "written": rep.written.iter().map(|(id, i)| json!({ "id": id, "state": i })).collect::<Vec<_>>(),
            "skipped": skipped,
            "sector_history": rep.entries.iter().map(|i| json!({ "index": i, "sector": label(&m, m.sector_log[*i].sector) })).collect::<Vec<_>>(),
        })))
    }

    fn write_sector(&mut self, a: &Value) -> Result<Value, String> {
        let ws = self.workspace()?;
        let m = ws.read().map_err(|e| e.to_string())?;
        let l = arg_u64(a, "sector")?;
        let s = sector_by_label(&m, l)?;
        let img = read_png(arg_str(a, "path")?)?;
        let keep = a.get("keep_alpha").and_then(Value::as_bool).unwrap_or(true);
        let (m, rep) = ws.write_sector(s, &img, arg_str(a, "name")?, &self.client, keep, arg_filter(a)?).map_err(|e| e.to_string())?;
        Ok(text(json!({
            "written": rep.written.iter().map(|(id, i)| json!({ "id": id, "state": i })).collect::<Vec<_>>(),
            "skipped": rep.skipped.iter().map(|(id, r)| json!({ "id": id, "reason": r })).collect::<Vec<_>>(),
            "sector_history": rep.entries.iter().map(|i| json!({ "index": i, "sector": label(&m, m.sector_log[*i].sector) })).collect::<Vec<_>>(),
        })))
    }

    fn go_to_state(&self, a: &Value) -> Result<Value, String> {
        let ws = self.workspace()?;
        if let Some(entry) = a.get("entry").and_then(Value::as_u64) {
            let before = a.get("before").and_then(Value::as_bool).unwrap_or(false);
            let (_, skipped) = ws.mutate(|m| m.go_to_entry(entry as usize, before)).map_err(|e| e.to_string())?;
            return Ok(text(json!({ "entry": entry, "before": before, "skipped": skipped })));
        }
        let id = arg_u64(a, "tile")?;
        let i = arg_u64(a, "state")? as usize;
        ws.mutate(|m| m.go_to_state(id, i)).map_err(|e| e.to_string())?;
        Ok(text(json!({ "tile": id, "current": i })))
    }

    /// Writes a layout proposal (design D7). Only layout edits are accepted, and they must apply
    /// to the workspace as it is now.
    fn propose(&self, a: &Value) -> Result<Value, String> {
        let ws = self.workspace()?;
        let m = ws.read().map_err(|e| e.to_string())?;
        let text_ = arg_str(a, "text")?;
        let list = a.get("edits").and_then(Value::as_array).ok_or("'edits' (a list) is required")?;
        let mut edits = Vec::new();
        let mut trial = m.clone();
        for (k, v) in list.iter().enumerate() {
            let e: Edit = serde_json::from_value(v.clone()).map_err(|e| format!("edit {}: {e}", k + 1))?;
            let allowed = matches!(
                e,
                Edit::Move { .. } | Edit::Scale { .. } | Edit::Rotate { .. } | Edit::Turn { .. } | Edit::SetPivot { .. } | Edit::Delete { .. } | Edit::AddSector { .. } | Edit::RemoveSector { .. } | Edit::AddSectors { .. } | Edit::RemoveSectors { .. }
            );
            if !allowed {
                return Err(format!("edit {}: only move, scale, rotate, turn, set_pivot, delete and sector edits can be proposed", k + 1));
            }
            let ids: Vec<u64> = match &e {
                Edit::Move { ids, .. } | Edit::Rotate { ids, .. } | Edit::Delete { ids } => ids.clone(),
                Edit::Scale { id, .. } | Edit::Turn { id, .. } | Edit::SetPivot { id, .. } => vec![*id],
                _ => Vec::new(),
            };
            if let Some(id) = ids.iter().find(|id| trial.tile(**id).is_none()) {
                return Err(format!("edit {}: there is no tile {id}", k + 1));
            }
            edit::apply(&mut trial, e.clone()).map_err(|r| format!("edit {}: {}", k + 1, r.0))?;
            edits.push(e);
        }
        if edits.is_empty() {
            return Err("a proposal needs at least one edit".into());
        }
        let p = Proposal { text: text_.into(), by: self.client.clone(), at: crate::atlas::model::now(), base_revision: m.revision, edits };
        write_file(&ws.dir.join(PROPOSAL), serde_json::to_string_pretty(&p).unwrap().as_bytes())?;
        Ok(text(json!({ "proposed": p.edits.len(), "note": "Shown in the ripper ATLAS window as a preview; the user applies or discards it. A newer proposal replaces this one." })))
    }
}

/// Removes temp files older than a day; other servers may be using the newer ones.
fn clean_old(dir: &Path) {
    let day = std::time::Duration::from_secs(24 * 3600);
    let Ok(ws) = std::fs::read_dir(dir) else { return };
    for w in ws.flatten() {
        let Ok(files) = std::fs::read_dir(w.path()) else { continue };
        for f in files.flatten() {
            let old = f.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|e| e > day);
            if old {
                let _ = std::fs::remove_file(f.path());
            }
        }
    }
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes).and_then(|_| std::fs::rename(&tmp, path)).map_err(|e| e.to_string())
}

/// The tool list with JSON Schemas (design D3).
pub fn tools() -> Value {
    let filter = json!({ "type": "string", "enum": ["smooth", "nearest"], "description": "How ripper scales the picture into the tile's frame: nearest for pixel art, smooth otherwise. Default smooth." });
    let keep = json!({ "type": "boolean", "description": "Multiply the new picture's alpha by the original's alpha (keeps the cut-out shape). Default true." });
    json!([
        { "name": "atlas_info", "description": "The current ripper workspace: sectors (by UDIM label), tiles with their frames and picture sizes, sector history entries, and whether a ripper window is open.",
          "inputSchema": { "type": "object", "properties": {} } },
        { "name": "get_scope", "description": "Which tiles to work on. Without target it follows the selection in the open ripper window: selected tiles, else selected sectors, else the whole atlas with confirm=true (ask the user first). The result is frozen under scope_id.",
          "inputSchema": { "type": "object", "properties": { "target": { "type": "object", "description": "Explicit target named by the user; wins over the selection.", "properties": {
              "tiles": { "type": "array", "items": { "type": "integer" } }, "sectors": { "type": "array", "items": { "type": "integer" }, "description": "UDIM labels such as 1001" }, "all": { "type": "boolean" } } } } } },
        { "name": "get_tile", "description": "A tile's current picture as a full-size PNG path (a copy you may read), a thumbnail, and its picture history.",
          "inputSchema": { "type": "object", "properties": { "id": { "type": "integer" } }, "required": ["id"] } },
        { "name": "render_sector", "description": "Renders a whole sector at full size to a PNG path, with each tile's box in it and whether a whole-sector write can cut it back.",
          "inputSchema": { "type": "object", "properties": { "sector": { "type": "integer", "description": "UDIM label" } }, "required": ["sector"] } },
        { "name": "write_tile", "description": "Writes new pictures for tiles from PNG paths. Each becomes the tile's current state; the frame stays and earlier pictures stay in its history. Agree the plan with the user first.",
          "inputSchema": { "type": "object", "properties": {
              "tiles": { "type": "array", "items": { "type": "object", "properties": { "id": { "type": "integer" }, "path": { "type": "string" } }, "required": ["id", "path"] } },
              "name": { "type": "string", "description": "Short name of the edit for the history, e.g. 'Pixelize 64'" },
              "scope_id": { "type": "string" }, "keep_alpha": keep, "filter": filter }, "required": ["tiles", "name"] } },
        { "name": "write_sector", "description": "Writes back an edited render_sector picture (same size); ripper cuts it into the sector's tiles by their boxes. Tilted tiles and tiles crossing the sector edge are skipped and named.",
          "inputSchema": { "type": "object", "properties": { "sector": { "type": "integer" }, "path": { "type": "string" }, "name": { "type": "string" }, "keep_alpha": keep, "filter": filter }, "required": ["sector", "path", "name"] } },
        { "name": "go_to_state", "description": "Makes an earlier or later picture state current: a tile's state index (from get_tile), or a sector history entry (from atlas_info), with before=true for the state before that entry.",
          "inputSchema": { "type": "object", "properties": { "tile": { "type": "integer" }, "state": { "type": "integer" }, "entry": { "type": "integer" }, "before": { "type": "boolean" } } } },
        { "name": "propose", "description": "Proposes layout changes; nothing changes until the user presses APPLY in ripper. Edits use ripper's edit format: {op:'move', ids, dx, dy}, {op:'scale', id, w, anchor:'top_left'|'pivot'|...}, {op:'rotate', ids, clockwise}, {op:'turn', id, angle}, {op:'set_pivot', id, pivot}, {op:'delete', ids}, {op:'add_sectors', cells:[{u,v}]}, {op:'remove_sectors', cells}. Canvas pixels, y down; sector (u,v) spans x u*S..(u+1)*S, y -(v+1)*S..-v*S.",
          "inputSchema": { "type": "object", "properties": { "text": { "type": "string", "description": "What and why, shown to the user" }, "edits": { "type": "array", "items": { "type": "object" } } }, "required": ["text", "edits"] } },
    ])
}

/// Runs the server on stdin / stdout until stdin closes.
pub fn run_stdio() {
    use std::io::{BufRead, Write};
    let mut server = Server::for_machine();
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(msg) => server.handle(&msg),
            Err(e) => Some(json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": format!("Parse error: {e}") } })),
        };
        if let Some(r) = reply {
            let _ = writeln!(out, "{r}");
            let _ = out.flush();
        }
    }
}
