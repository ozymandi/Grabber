//! Tauri commands for ATLAS mode (OpenSpec change add-texture-atlas). The model lives in
//! `ripper_core::atlas`; every command works on the latest manifest on disk and returns the
//! resulting document, so the page never holds authoritative state (design D3).

use crate::AppState;
use ripper_core::atlas::edit::{undo_guide_load, Edit};
use ripper_core::atlas::model::Sector;
use ripper_core::atlas::render::Scope;
use ripper_core::atlas::store::{Doc, Error, Workspace};
use ripper_core::atlas::files;
use ripper_core::settings;
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

#[derive(serde::Serialize)]
pub struct AtlasName {
    /// None when there is no current workspace yet.
    pub name: Option<String>,
    /// The `.ripper` file, if the workspace was saved; its name then follows the file.
    pub file: Option<String>,
}

/// Name of the current workspace for the RIP Output panel, without creating one.
#[tauri::command]
pub fn atlas_name(state: State<'_, AppState>) -> AtlasName {
    let id = state.settings.lock().unwrap().current_workspace.clone();
    match state.store.existing(&id) {
        Some(ws) => AtlasName { name: ws.read().ok().map(|m| m.name), file: state.store.entry(&ws.id).file },
        None => AtlasName { name: None, file: None },
    }
}

/// Names an unsaved workspace (2.1.2). A saved workspace is named after its file, so it is
/// renamed with SAVE AS instead.
#[tauri::command]
pub async fn atlas_rename(name: String, state: State<'_, AppState>) -> Result<Doc, String> {
    let ws = state.current_workspace().map_err(err)?;
    if state.store.entry(&ws.id).file.is_some() {
        return Err("A saved workspace is named after its file; use SAVE AS in ATLAS to rename it".into());
    }
    let w = ws.clone();
    async_runtime::spawn_blocking(move || w.mutate(|m| m.rename(&name)))
        .await
        .map_err(|e| e.to_string())?
        .map_err(err)?;
    state.store.doc(&ws).map_err(err)
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
    state.held.lock().unwrap().clear();
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
    state.held.lock().unwrap().clear();
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
    let keep = state.held.lock().unwrap().clone();
    async_runtime::spawn_blocking(move || store.save_as_keeping(&ws, &PathBuf::from(file), &keep))
        .await
        .map_err(|e| e.to_string())?
        .map(Some)
        .map_err(err)
}

#[tauri::command]
pub async fn atlas_save_as(path: String, state: State<'_, AppState>) -> Result<Doc, String> {
    let ws = state.current_workspace().map_err(err)?;
    let store = state.store.clone();
    let keep = state.held.lock().unwrap().clone();
    async_runtime::spawn_blocking(move || store.save_as_keeping(&ws, &PathBuf::from(path), &keep))
        .await
        .map_err(|e| e.to_string())?
        .map_err(err)
}

#[derive(serde::Serialize)]
pub struct EditResult {
    pub doc: Doc,
    /// Tiles removed by a Delete.
    pub removed: usize,
    /// Edits that undo this one; empty for settings, which are not part of the history.
    pub inverse: Vec<Edit>,
    pub label: String,
}

impl AppState {
    fn hold(&self, items: Vec<String>) {
        self.held.lock().unwrap().extend(items);
    }
}

#[tauri::command]
pub async fn atlas_edit(edit: Edit, state: State<'_, AppState>) -> Result<EditResult, String> {
    let ws = state.current_workspace().map_err(err)?;
    let w = ws.clone();
    let (_, a) = async_runtime::spawn_blocking(move || w.edit(edit))
        .await
        .map_err(|e| e.to_string())?
        .map_err(err)?;
    state.hold(a.released);
    Ok(EditResult { doc: state.store.doc(&ws).map_err(err)?, removed: a.removed, inverse: a.inverse, label: a.label })
}

#[derive(serde::Serialize)]
pub struct StepsResult {
    pub doc: Doc,
    /// The edits that reverse what was applied (the redo of an undo, and the other way round).
    pub inverse: Vec<Edit>,
    /// Why parts of the step could not be applied.
    pub skipped: Vec<String>,
}

/// Undo and redo: applies a step best effort (design D6 of add-atlas-fine-work).
#[tauri::command]
pub async fn atlas_apply_steps(edits: Vec<Edit>, state: State<'_, AppState>) -> Result<StepsResult, String> {
    let ws = state.current_workspace().map_err(err)?;
    let w = ws.clone();
    let (_, inverse, skipped, released) = async_runtime::spawn_blocking(move || w.apply_steps(edits))
        .await
        .map_err(|e| e.to_string())?
        .map_err(err)?;
    state.hold(released);
    Ok(StepsResult { doc: state.store.doc(&ws).map_err(err)?, inverse, skipped })
}

#[derive(serde::Serialize)]
pub struct GuidesResult {
    pub doc: Doc,
    pub loaded: usize,
    pub created: usize,
    pub not_square: usize,
    pub inverse: Vec<Edit>,
    pub label: String,
}

/// LOAD in the Guides panel: reads the chosen PNGs and places them (design D4).
#[tauri::command]
pub async fn atlas_load_guides(paths: Vec<String>, target: Option<Sector>, state: State<'_, AppState>) -> Result<GuidesResult, String> {
    let ws = state.current_workspace().map_err(err)?;
    let w = ws.clone();
    let (_, rep) = async_runtime::spawn_blocking(move || {
        let mut files = Vec::new();
        for p in &paths {
            let path = PathBuf::from(p);
            let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let bytes = std::fs::read(&path).map_err(|e| Error::Refused(format!("{name}: {e}")))?;
            files.push((name, bytes));
        }
        w.load_guides(&files, target)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(err)?;
    let released: Vec<String> = rep.replaced.iter().filter_map(|(_, g)| g.as_ref().map(|g| g.item.clone())).collect();
    state.hold(released);
    let label = if rep.loaded == 1 { "load guide".to_string() } else { format!("load {} guides", rep.loaded) };
    Ok(GuidesResult {
        doc: state.store.doc(&ws).map_err(err)?,
        loaded: rep.loaded,
        created: rep.created.len(),
        not_square: rep.not_square,
        inverse: undo_guide_load(&rep.replaced, &rep.created),
        label,
    })
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
