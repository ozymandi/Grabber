//! MCP server (OpenSpec change add-mcp-editing, tasks 3.1-3.3): the stdio protocol through the
//! real `ripper-mcp` binary, and the tools and scope rules through the in-process server.

use ripper_core::atlas::edit::Edit;
use ripper_core::atlas::files::{decode_png, encode_png};
use ripper_core::atlas::model::{Frame, Rect, Sector};
use ripper_core::atlas::render::Image;
use ripper_core::atlas::store::{Store, Workspace};
use ripper_core::mcp::{now_ms, Proposal, Server, UiState, PROPOSAL, UI_STATE};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn noise(w: u32, h: u32, seed: u32) -> Image {
    let mut x = seed.wrapping_mul(2654435761).max(1);
    let mut rgba = Vec::new();
    for _ in 0..w * h {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        rgba.extend_from_slice(&[x as u8, (x >> 8) as u8, (x >> 16) as u8, 255]);
    }
    Image { w, h, rgba }
}

/// A home folder like `%APPDATA%\ripper` with one workspace of `n` tiles as the current one.
fn home(tag: &str, n: u32) -> (PathBuf, Store, Workspace) {
    let h = std::env::temp_dir().join(format!("ripper-mcp-test-{tag}-{}", now_ms()));
    let _ = std::fs::remove_dir_all(&h);
    std::fs::create_dir_all(&h).unwrap();
    let store = Store::new(h.join("workspaces"));
    let ws = store.create().unwrap();
    for k in 0..n {
        let o = noise(64, 32, k + 1);
        ws.add_tile(&encode_png(&o), o.w, o.h, Default::default()).unwrap();
    }
    let s = ripper_core::settings::Settings { current_workspace: ws.id.clone(), ..Default::default() };
    ripper_core::settings::save(&h.join("config.toml"), &s).unwrap();
    (h, store, ws)
}

fn server(h: &std::path::Path) -> Server {
    let mut s = Server::new(Store::new(h.join("workspaces")), h.join("config.toml"), h.join("temp"));
    s.handle(&json!({ "jsonrpc": "2.0", "id": 0, "method": "initialize", "params": { "protocolVersion": "2025-06-18", "clientInfo": { "name": "claude-code" } } }));
    s
}

/// Calls a tool; returns (structuredContent, isError, whole result).
fn call(s: &mut Server, name: &str, args: Value) -> (Value, bool, Value) {
    let r = s.handle(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": name, "arguments": args } })).unwrap();
    let res = r["result"].clone();
    (res["structuredContent"].clone(), res["isError"].as_bool().unwrap_or(false), res)
}

fn publish(ws: &Workspace, tiles: &[u64], sectors: &[Sector], at: u64) {
    let u = UiState { tiles: tiles.to_vec(), sectors: sectors.to_vec(), at };
    std::fs::write(ws.dir.join(UI_STATE), serde_json::to_string(&u).unwrap()).unwrap();
}

fn ids(v: &Value) -> Vec<u64> {
    v["tiles"].as_array().unwrap().iter().map(|t| t["id"].as_u64().unwrap()).collect()
}

struct Client {
    stdin: std::process::ChildStdin,
    out: BufReader<std::process::ChildStdout>,
}

impl Client {
    fn send(&mut self, line: &str) {
        writeln!(self.stdin, "{line}").unwrap();
        self.stdin.flush().unwrap();
    }
    fn ask(&mut self, line: &str) -> Value {
        self.send(line);
        let mut s = String::new();
        self.out.read_line(&mut s).unwrap();
        serde_json::from_str(&s).unwrap()
    }
}

#[test]
fn stdio_transcript() {
    let (h, _, ws) = home("stdio", 2);
    let mut p = Command::new(env!("CARGO_BIN_EXE_ripper-mcp"))
        .env("RIPPER_MCP_HOME", &h)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut c = Client { stdin: p.stdin.take().unwrap(), out: BufReader::new(p.stdout.take().unwrap()) };
    let init = c.ask(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","clientInfo":{"name":"codex"}}}"#);
    assert_eq!(init["id"], 1);
    assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
    assert!(init["result"]["capabilities"]["tools"].is_object());
    assert!(init["result"]["instructions"].as_str().unwrap().contains("get_scope"));
    // A notification gets no reply: the next line read answers the ping.
    c.send(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
    let ping = c.ask(r#"{"jsonrpc":"2.0","id":2,"method":"ping"}"#);
    assert_eq!(ping, json!({ "jsonrpc": "2.0", "id": 2, "result": {} }));
    let list = c.ask(r#"{"jsonrpc":"2.0","id":3,"method":"tools/list"}"#);
    let names: Vec<&str> = list["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["atlas_info", "get_scope", "get_tile", "render_sector", "write_tile", "write_sector", "go_to_state", "propose"]);
    let info = c.ask(r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"atlas_info","arguments":{}}}"#);
    assert_eq!(info["result"]["structuredContent"]["workspace"]["id"], ws.id.as_str());
    assert_eq!(info["result"]["structuredContent"]["tiles"].as_array().unwrap().len(), 2);
    let bad = c.ask(r#"{"jsonrpc":"2.0","id":5,"method":"resources/list"}"#);
    assert_eq!(bad["error"]["code"], -32601);
    let tool_err = c.ask(r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"get_tile","arguments":{"id":99}}}"#);
    assert_eq!(tool_err["result"]["isError"], true);
    let garbage = c.ask("{not json");
    assert_eq!(garbage["error"]["code"], -32700);
    drop(c);
    assert!(p.wait().unwrap().success());
}

#[test]
fn tools_read_write_and_history() {
    let (h, _, ws) = home("tools", 3);
    let mut s = server(&h);
    let (info, err, _) = call(&mut s, "atlas_info", json!({}));
    assert!(!err);
    let all = ids(&info);
    assert_eq!(info["sectors"][0]["label"], 1001);
    assert_eq!(info["window_open"], false);

    // get_tile: a full-size copy and a thumbnail no longer than 256 px.
    let (t, err, whole) = call(&mut s, "get_tile", json!({ "id": all[0] }));
    assert!(!err);
    let pic = decode_png(&std::fs::read(t["path"].as_str().unwrap()).unwrap()).unwrap();
    assert_eq!((pic.w, pic.h), (64, 32));
    let img = whole["content"].as_array().unwrap().iter().find(|c| c["type"] == "image").unwrap();
    assert_eq!(img["mimeType"], "image/png");
    assert_eq!(t["states"].as_array().unwrap().len(), 1);

    // write_tile: a new state named and authored.
    let new = h.join("new.png");
    std::fs::write(&new, encode_png(&noise(16, 8, 7))).unwrap();
    let (w, err, _) = call(&mut s, "write_tile", json!({ "tiles": [{ "id": all[0], "path": new.display().to_string() }], "name": "Pixelize 16", "filter": "nearest" }));
    assert!(!err, "{w}");
    assert_eq!(w["written"][0]["state"], 1);
    let m = ws.read().unwrap();
    let st = &m.tile(all[0]).unwrap().history[1];
    assert_eq!((st.name.as_str(), st.by.as_str(), st.w0), ("Pixelize 16", "Claude Code", 16));

    // go_to_state back to the original.
    let (_, err, _) = call(&mut s, "go_to_state", json!({ "tile": all[0], "state": 0 }));
    assert!(!err);
    assert_eq!(ws.read().unwrap().tile(all[0]).unwrap().w0, 64);

    // render_sector + write_sector: the unchanged render gives every tile a state.
    let (r, err, _) = call(&mut s, "render_sector", json!({ "sector": 1001 }));
    assert!(!err, "{r}");
    let (ws_out, err, _) = call(&mut s, "write_sector", json!({ "sector": 1001, "path": r["path"], "name": "Same" }));
    assert!(!err, "{ws_out}");
    assert_eq!(ws_out["written"].as_array().unwrap().len(), 3);
    assert_eq!(ws_out["sector_history"][0]["sector"], 1001);
    // go_to_state on the sector entry, before it.
    let (_, err, _) = call(&mut s, "go_to_state", json!({ "entry": 0, "before": true }));
    assert!(!err);

    // Errors are tool errors, not crashes.
    let (_, err, _) = call(&mut s, "write_tile", json!({ "tiles": [{ "id": all[1], "path": "C:/nope.png" }], "name": "x" }));
    assert!(!err, "a missing file is a skipped tile, reported");
    let (_, err, _) = call(&mut s, "render_sector", json!({ "sector": 1002 }));
    assert!(err);
}

#[test]
fn propose_writes_only_layout() {
    let (h, _, ws) = home("propose", 2);
    let mut s = server(&h);
    let all: Vec<u64> = ws.read().unwrap().tiles.iter().map(|t| t.id).collect();
    let (p, err, _) = call(&mut s, "propose", json!({ "text": "Move tile into the corner", "edits": [{ "op": "move", "ids": [all[0]], "dx": 10, "dy": 5 }] }));
    assert!(!err, "{p}");
    let saved: Proposal = serde_json::from_str(&std::fs::read_to_string(ws.dir.join(PROPOSAL)).unwrap()).unwrap();
    assert_eq!(saved.edits, vec![Edit::Move { ids: vec![all[0]], dx: 10, dy: 5 }]);
    assert_eq!(saved.by, "Claude Code");
    // Nothing moved yet.
    let before = ws.read().unwrap();
    assert_eq!(before.tiles[0].frame, ws.read().unwrap().tiles[0].frame);
    let (_, err, _) = call(&mut s, "propose", json!({ "text": "x", "edits": [{ "op": "set_size", "size": 512 }] }));
    assert!(err, "settings are not proposals");
    let (_, err, _) = call(&mut s, "propose", json!({ "text": "x", "edits": [{ "op": "delete", "ids": [999] }] }));
    assert!(err, "an edit that does not apply is refused");
}

#[test]
fn scope_follows_the_selection() {
    let (h, _, ws) = home("scope", 4);
    let mut s = server(&h);
    // Put tiles 3 and 4 into a second sector.
    ws.edit(Edit::AddSector { sector: Sector { u: 0, v: 0 }, side: ripper_core::atlas::model::Side::Right }).unwrap();
    let all: Vec<u64> = ws.read().unwrap().tiles.iter().map(|t| t.id).collect();
    let size = ws.read().unwrap().sector_size as i64;
    for (k, id) in all[2..].iter().enumerate() {
        ws.edit(Edit::SetFrame { id: *id, frame: Frame::from_rect(Rect { x: size + 10 + 100 * k as i64, y: -200, w: 64, h: 32 }) }).unwrap();
    }

    // No window: whole atlas, confirm.
    let (sc, _, _) = call(&mut s, "get_scope", json!({}));
    assert_eq!((sc["source"].as_str(), sc["confirm"].as_bool(), sc["window_open"].as_bool()), (Some("atlas"), Some(true), Some(false)));
    assert_eq!(ids(&sc).len(), 4);

    // Three tiles selected -> exactly those.
    publish(&ws, &[all[0], all[1], all[3]], &[], now_ms());
    let (sc, _, _) = call(&mut s, "get_scope", json!({}));
    assert_eq!((sc["source"].as_str(), ids(&sc)), (Some("tiles"), vec![all[0], all[1], all[3]]));
    let frozen = sc["scope_id"].as_str().unwrap().to_string();

    // Tiles win over sectors.
    publish(&ws, &[all[1]], &[Sector { u: 1, v: 0 }], now_ms());
    let (sc, _, _) = call(&mut s, "get_scope", json!({}));
    assert_eq!(ids(&sc), vec![all[1]]);

    // Sector selected -> its tiles.
    publish(&ws, &[], &[Sector { u: 1, v: 0 }], now_ms());
    let (sc, _, _) = call(&mut s, "get_scope", json!({}));
    assert_eq!((sc["source"].as_str(), ids(&sc), sc["sectors"].clone()), (Some("sectors"), vec![all[2], all[3]], json!([1002])));
    let sector_scope = sc["scope_id"].as_str().unwrap().to_string();

    // An explicit target wins over the selection.
    let (sc, _, _) = call(&mut s, "get_scope", json!({ "target": { "sectors": [1001] } }));
    assert_eq!(ids(&sc), vec![all[0], all[1]]);

    // Nothing selected in an open window -> atlas, confirm.
    publish(&ws, &[], &[], now_ms());
    let (sc, _, _) = call(&mut s, "get_scope", json!({}));
    assert_eq!((sc["source"].as_str(), sc["confirm"].as_bool(), sc["window_open"].as_bool()), (Some("atlas"), Some(true), Some(true)));

    // A stale file means no window.
    publish(&ws, &[all[0]], &[], now_ms() - 6000);
    let (sc, _, _) = call(&mut s, "get_scope", json!({}));
    assert_eq!((sc["source"].as_str(), sc["window_open"].as_bool()), (Some("atlas"), Some(false)));

    // The frozen scope still holds three tiles after the selection changed; tile 3 is outside it.
    let png = h.join("p.png");
    std::fs::write(&png, encode_png(&noise(8, 8, 3))).unwrap();
    let p = png.display().to_string();
    let (w, _, _) = call(&mut s, "write_tile", json!({ "scope_id": frozen, "name": "x", "tiles": [{ "id": all[0], "path": p }, { "id": all[2], "path": p }] }));
    assert_eq!(w["written"].as_array().unwrap().len(), 1);
    assert_eq!(w["skipped"][0]["reason"], "not in the scope");
    assert!(w["sector_history"].as_array().unwrap().is_empty(), "a tile scope makes no sector entry");

    // A sector scope makes one sector entry.
    let (w, _, _) = call(&mut s, "write_tile", json!({ "scope_id": sector_scope, "name": "Pixelize 1002", "tiles": [{ "id": all[2], "path": p }, { "id": all[3], "path": p }] }));
    assert_eq!(w["sector_history"], json!([{ "index": 0, "sector": 1002 }]));
}
