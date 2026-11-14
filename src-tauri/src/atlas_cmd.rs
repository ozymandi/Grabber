//! Tauri commands for ATLAS mode (OpenSpec change add-texture-atlas). The model lives in
//! `ripper_core::atlas`; every command works on the latest manifest on disk and returns the
//! resulting document, so the page never holds authoritative state (design D3).

use crate::AppState;
use ripper_core::atlas::model::{Corner, Refused, Sector, Side};
use ripper_core::atlas::render::Scope;
use ripper_core::atlas::store::{Doc, Error, Workspace};
use ripper_core::atlas::files;
use ripper_core::settings;
use serde::Deserialize;
use std::path::PathBuf;
use tauri::{async_runtime, State};

fn err(e: Error) -> String {
    e.to_string()
}

impl AppState {
    /// The current workspace, created (and remembered in the settings) when there is none.
    pub fn current_workspace(&self) -> Result<Workspace, Error> {
        let id = self.settings.lock().unwrap().current_workspace.clone();
        if let Some(ws) = self.store.existing(&id) {
            return Ok(ws);
        }
        let ws = self.store.create()?;
        self.set_current(&ws);
        Ok(ws)
    }

    pub fn set_current(&self, ws: &Workspace) {
        let mut s = self.settings.lock().unwrap();
        s.current_workspace = ws.id.clone();
        let _ = settings::save(&self.settings_path, &s);
    }
}

/// The current workspace document.
#[tauri::command]
pub async fn atlas_get(state: State<'_, AppState>) -> Result<Doc, String> {
    let ws = state.current_workspace().map_err(err)?;
    state.store.doc(&ws).map_err(err)
}

/// Name of the current workspace for the RIP Output panel, without creating one.
#[tauri::command]
pub fn atlas_name(state: State<'_, AppState>) -> Option<String> {
    let id = state.settings.lock().unwrap().current_workspace.clone();
    state.store.existing(&id).and_then(|ws| ws.read().ok()).map(|m| m.name)
}

/// Revision of the current workspace, polled by the open atlas view.
#[tauri::command]
pub fn atlas_revision(state: State<'_, AppState>) -> Result<u64, String> {
    let ws = state.current_workspace().map_err(err)?;
    ws.revision().map_err(err)
}

#[tauri::command]
pub async fn atlas_new(state: State<'_, AppState>) -> Result<Doc, String> {
    let ws = state.store.create().map_err(err)?;
    state.set_current(&ws);
    state.store.doc(&ws).map_err(err)
}

#[tauri::command]
pub async fn atlas_open(path: String, state: State<'_, AppState>) -> Result<Doc, String> {
    let store = state.store.clone();
    let ws = async_runtime::spawn_blocking(move || store.open(&PathBuf::from(path)))
        .await
        .map_err(|e| e.to_string())?
        .map_err(err)?;
    state.set_current(&ws);
    state.store.doc(&ws).map_err(err)
}

/// Saves to the workspace's file. An untitled workspace returns `Ok(None)` so the page
/// asks for a path and calls `atlas_save_as`.
#[tauri::command]
pub async fn atlas_save(state: State<'_, AppState>) -> Result<Option<Doc>, String> {
    let ws = state.current_workspace().map_err(err)?;
    let Some(file) = state.store.entry(&ws.id).file else { return Ok(None) };
    let store = state.store.clone();
    async_runtime::spawn_blocking(move || store.save_as(&ws, &PathBuf::from(file)))
        .await
        .map_err(|e| e.to_string())?
        .map(Some)
        .map_err(err)
}

#[tauri::command]
pub async fn atlas_save_as(path: String, state: State<'_, AppState>) -> Result<Doc, String> {
    let ws = state.current_workspace().map_err(err)?;
    let store = state.store.clone();
    async_runtime::spawn_blocking(move || store.save_as(&ws, &PathBuf::from(path)))
        .await
        .map_err(|e| e.to_string())?
        .map_err(err)
}

/// One edit of the current workspace.
#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Edit {
    SetSize { size: u32 },
    SetPadding { padding: u32 },
    AddSector { sector: Sector, side: Side },
    RemoveSector { sector: Sector },
    SetActive { sector: Sector },
    Move { moves: Vec<(u64, i64, i64)> },
    Scale { id: u64, w: i64, anchor: Corner },
    Rotate { ids: Vec<u64>, clockwise: bool },
    Delete { ids: Vec<u64> },
}

#[derive(serde::Serialize)]
pub struct EditResult {
    pub doc: Doc,
    /// Tiles removed by a Delete.
    pub removed: usize,
}

#[tauri::command]
pub async fn atlas_edit(edit: Edit, state: State<'_, AppState>) -> Result<EditResult, String> {
    let ws = state.current_workspace().map_err(err)?;
    let (_, removed) = async_runtime::spawn_blocking(move || {
        ws.mutate(|m| -> Result<usize, Refused> {
            Ok(match edit {
                Edit::SetSize { size } => m.set_sector_size(size).map(|_| 0)?,
                Edit::SetPadding { padding } => m.set_padding(padding).map(|_| 0)?,
                Edit::AddSector { sector, side } => m.add_sector(sector, side).map(|_| 0)?,
                Edit::RemoveSector { sector } => m.remove_sector(sector).map(|_| 0)?,
                Edit::SetActive { sector } => m.set_active(sector).map(|_| 0)?,
                Edit::Move { moves } => m.move_tiles(&moves).map(|_| 0)?,
                Edit::Scale { id, w, anchor } => m.scale_tile(id, w, anchor).map(|_| 0)?,
                Edit::Rotate { ids, clockwise } => m.rotate_tiles(&ids, clockwise).map(|_| 0)?,
                Edit::Delete { ids } => m.delete_tiles(&ids),
            })
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(err)?;
    let ws = state.current_workspace().map_err(err)?;
    Ok(EditResult { doc: state.store.doc(&ws).map_err(err)?, removed })
}

#[tauri::command]
pub async fn atlas_export(scope: Scope, sectors: Vec<Sector>, dir: String, base: String, state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let ws = state.current_workspace().map_err(err)?;
    async_runtime::spawn_blocking(move || files::export(&ws, &sectors, scope, &PathBuf::from(dir), &base))
        .await
        .map_err(|e| e.to_string())?
        .map(|v| v.into_iter().map(|p| p.display().to_string()).collect())
        .map_err(err)
}

/// `ripper://atlas/<workspace>/<item>?max=N`: a display copy of an original.
pub fn serve_thumbnail(state: &AppState, path: &str, query: Option<&str>) -> Option<Vec<u8>> {
    let mut parts = path.trim_start_matches('/').splitn(3, '/');
    if parts.next()? != "atlas" {
        return None;
    }
    let ws = state.store.existing(parts.next()?)?;
    let item = parts.next()?;
    let max = query
        .and_then(|q| q.split('&').find_map(|kv| kv.strip_prefix("max=")))
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(2048)
        .clamp(16, 8192);
    if !item.ends_with(".png") || item.contains(['/', '\\']) || item.starts_with('.') {
        return None;
    }
    files::thumbnail(&ws, item, max).ok()
}
