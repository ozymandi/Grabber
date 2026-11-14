// Release builds use the Windows GUI subsystem so no console window appears.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod atlas_cmd;
mod update;

use ripper_core::export::{self, Failure, Outcome, QuadReq};
use ripper_core::geom::Invalid;
use ripper_core::launch::{self, Launch, Mode};
use ripper_core::settings::{self, Settings};
use ripper_core::source::{self, Source};
use ripper_core::atlas::model::Provenance;
use ripper_core::atlas::store::{self as atlas_store, Store};
use ripper_core::clipboard;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use tauri::async_runtime::{self, JoinHandle};
use tauri::http::{Response, StatusCode};
use tauri::{AppHandle, Manager, State};

/// Session state owned by Rust (design D3): launch mode, decoded source pixels, settings.
struct AppState {
    launch: Launch,
    source: RwLock<Option<Arc<Source>>>,
    /// Bumped on every new source so the UI can bust caches (`/source.rgba?rev=N`).
    rev: AtomicU64,
    /// Decode of the launch file, started before the WebView is up.
    startup: Mutex<Option<JoinHandle<Result<Source, String>>>>,
    settings: Mutex<Settings>,
    settings_path: PathBuf,
    /// Set when the settings file could not be read at start; shown once in the status row.
    settings_warning: Mutex<Option<String>>,
    /// Newer release found by the background check (standalone mode only).
    update: Mutex<Option<tauri_plugin_updater::Update>>,
    /// Atlas working folders under %APPDATA%/ripper/workspaces.
    store: Store,
    /// Items that left the current workspace but this window's undo history can bring back;
    /// SAVE keeps them on disk (design D6 of add-atlas-fine-work).
    held: Mutex<std::collections::HashSet<String>>,
}

#[derive(Serialize)]
struct SettingsInfo {
    settings: Settings,
    warning: Option<String>,
}

/// Current settings, plus a one-time warning if the file had to be reset.
#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> SettingsInfo {
    SettingsInfo {
        settings: state.settings.lock().unwrap().clone(),
        warning: state.settings_warning.lock().unwrap().take(),
    }
}

/// Replaces the settings and saves them. The window size is kept from the current value.
#[tauri::command]
fn set_settings(mut settings: Settings, state: State<'_, AppState>) -> Result<(), String> {
    let mut cur = state.settings.lock().unwrap();
    settings.window = cur.window;
    settings.maximized = cur.maximized;
    // The current workspace is owned by the atlas commands; a page copy may be stale.
    settings.current_workspace = cur.current_workspace.clone();
    *cur = settings;
    settings::save(&state.settings_path, &cur).map_err(|e| format!("Cannot save settings: {e}"))
}

#[derive(Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
enum ExportKind {
    /// Active quad to clipboard, ShareX output and folders (Enter).
    Send,
    /// Every quad to the folders (Ctrl+S).
    All,
    /// Active quad to the clipboard only.
    Copy,
}

#[derive(Deserialize)]
struct ExportReq {
    kind: ExportKind,
    quads: Vec<QuadReq>,
}

#[derive(Serialize)]
struct Skipped {
    number: u32,
    reason: Invalid,
}

#[derive(Serialize, Default)]
struct AtlasOutcome {
    /// Workspace name, for the status line.
    name: String,
    added: usize,
    /// Of the added tiles, how many went to the tray.
    tray: usize,
    error: Option<String>,
}

#[derive(Serialize)]
struct ExportResult {
    /// Files written to the output folders.
    written: Vec<String>,
    failures: Vec<Failure>,
    skipped: Vec<Skipped>,
    /// Number of enabled output folders.
    folders: usize,
    clipboard: bool,
    /// The ShareX output file, when it was written.
    output: Option<String>,
    /// Size of the sent quad, for the status line.
    size: Option<[u32; 2]>,
    /// ShareX mode and every write succeeded: the UI closes the app.
    exit_after: bool,
    /// What the Atlas output did; `None` when it is off.
    atlas: Option<AtlasOutcome>,
    /// Numbers of the quads that reached at least one destination (Send, Save All only).
    /// The UI clears their "unsaved" mark (design D6 of add-installer-updater).
    saved: Vec<u32>,
}

fn invalid_text(r: Invalid) -> &'static str {
    match r {
        Invalid::NotFinite => "invalid coordinates",
        Invalid::NotConvex => "edges cross or the shape is concave",
        Invalid::TooSmall => "too small",
        Invalid::Collinear => "three corners are nearly in a line",
        Invalid::EmptyShape => "the shape has no area",
    }
}

/// Exports quads (spec texture-export). Every output is attempted; failures are reported
/// per target and never stop the others.
#[tauri::command]
async fn export(req: ExportReq, state: State<'_, AppState>) -> Result<ExportResult, String> {
    let src = state.source.read().unwrap().clone().ok_or("No image loaded")?;
    let (folders, atlas_on) = {
        let s = state.settings.lock().unwrap();
        (s.folders(), s.atlas_on)
    };
    // Resolve (or create) the current workspace before rendering; a failure here is
    // reported as an atlas failure and never blocks the other outputs.
    let atlas_ws = if atlas_on { Some(state.current_workspace().map_err(|e| e.to_string())) } else { None };
    let sharex_output = match state.launch.mode() {
        Mode::Sharex => state.launch.output.clone(),
        Mode::Standalone => None,
    };
    async_runtime::spawn_blocking(move || {
        let mut out = Outcome::default();
        let mut res = ExportResult {
            written: Vec::new(),
            failures: Vec::new(),
            skipped: Vec::new(),
            folders: folders.len(),
            clipboard: false,
            output: None,
            size: None,
            exit_after: false,
            saved: Vec::new(),
            atlas: None,
        };
        // Rendered quads for the atlas: (png, w, h, number), added after every other output.
        let mut atlas_jobs: Vec<(Vec<u8>, u32, u32, u32)> = Vec::new();
        match req.kind {
            ExportKind::Send | ExportKind::Copy => {
                let q = req.quads.first().ok_or("No quad to send")?;
                let r = export::render(&src, q)
                    .map_err(|e| format!("Q{} cannot be exported: {}", q.number, invalid_text(e)))?;
                res.size = Some([r.w, r.h]);
                match clipboard::put_image(r.w, r.h, &r.rgba) {
                    Ok(()) => res.clipboard = true,
                    Err(e) => out.failures.push(Failure { target: "Clipboard".into(), path: None, error: e }),
                }
                if req.kind == ExportKind::Send {
                    let png = export::encode_png(&r);
                    if let Some(path) = &sharex_output {
                        match std::fs::write(path, &png) {
                            Ok(()) => res.output = Some(path.display().to_string()),
                            Err(e) => out.failures.push(Failure {
                                target: "ShareX output".into(),
                                path: Some(path.display().to_string()),
                                error: e.to_string(),
                            }),
                        }
                    }
                    let before = out.written.len();
                    export::write_to_folders(&folders, &src.stem, q.number, &png, &mut out);
                    if res.clipboard || res.output.is_some() || out.written.len() > before {
                        res.saved.push(q.number);
                    }
                    atlas_jobs.push((png, r.w, r.h, q.number));
                }
            }
            ExportKind::All => {
                for q in &req.quads {
                    match export::render(&src, q) {
                        Ok(r) => {
                            let before = out.written.len();
                            let png = export::encode_png(&r);
                            export::write_to_folders(&folders, &src.stem, q.number, &png, &mut out);
                            if out.written.len() > before {
                                res.saved.push(q.number);
                            }
                            atlas_jobs.push((png, r.w, r.h, q.number));
                        }
                        Err(reason) => res.skipped.push(Skipped { number: q.number, reason }),
                    }
                }
            }
        }
        if req.kind != ExportKind::Copy {
            if let Some(ws) = atlas_ws {
                let mut a = AtlasOutcome::default();
                match ws {
                    Err(e) => a.error = Some(e),
                    Ok(ws) => {
                        a.name = ws.read().map(|m| m.name).unwrap_or_default();
                        for (png, w, h, number) in &atlas_jobs {
                            match ws.add_tile(png, *w, *h, Provenance { stem: src.stem.clone(), quad: *number }) {
                                Ok((_, p)) => {
                                    a.added += 1;
                                    if p.sector.is_none() {
                                        a.tray += 1;
                                    }
                                    if !res.saved.contains(number) {
                                        res.saved.push(*number);
                                    }
                                }
                                Err(e) => {
                                    a.error = Some(match e {
                                        atlas_store::Error::Busy => "the atlas is busy in another ripper window".into(),
                                        e => e.to_string(),
                                    });
                                    break;
                                }
                            }
                        }
                    }
                }
                res.atlas = Some(a);
            }
        }
        let atlas_failed = res.atlas.as_ref().is_some_and(|a| a.error.is_some());
        res.exit_after = export::exit_after_send(req.kind == ExportKind::Send && sharex_output.is_some(), res.output.is_some(), out.failures.len(), atlas_failed);
        res.written = out.written;
        res.failures = out.failures;
        Ok(res)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Stores the main window's logical size in the settings file.
pub(crate) fn remember_window(app: &AppHandle) {
    let Some(w) = app.get_webview_window("main") else { return };
    if w.is_minimized().unwrap_or(false) {
        return;
    }
    let maximized = w.is_maximized().unwrap_or(false);
    let state = app.state::<AppState>();
    let mut s = state.settings.lock().unwrap();
    s.maximized = maximized;
    // A maximized window keeps the last normal size, so restoring it later looks right.
    if !maximized {
        let (Ok(size), Ok(scale)) = (w.inner_size(), w.scale_factor()) else { return };
        let logical = size.to_logical::<f64>(scale);
        s.window = Some([logical.width.round() as u32, logical.height.round() as u32]);
    }
    let _ = settings::save(&state.settings_path, &s);
}

/// Sizes the still hidden window from the settings so it fits its monitor's work area, centres
/// it there and maximizes it if it was maximized. tao applies the maximize only when the window
/// is shown, so there is no flash.
fn place_window(win: &tauri::WebviewWindow, saved: Option<[u32; 2]>, maximized: bool) {
    let monitor = win.current_monitor().ok().flatten().or_else(|| win.primary_monitor().ok().flatten());
    let Some(mon) = monitor else { return };
    let scale = mon.scale_factor();
    let wa = *mon.work_area();
    let (Ok(inner), Ok(outer)) = (win.inner_size(), win.outer_size()) else { return };
    let frame = [
        outer.width.saturating_sub(inner.width) as f64 / scale,
        outer.height.saturating_sub(inner.height) as f64 / scale,
    ];
    let work = [wa.size.width as f64 / scale, wa.size.height as f64 / scale];
    let [w, h] = settings::fit_window(saved, [1440.0, 900.0], work, frame);
    let _ = win.set_size(tauri::LogicalSize::new(w, h));
    if let Ok(outer) = win.outer_size() {
        let x = wa.position.x + (wa.size.width.saturating_sub(outer.width) / 2) as i32;
        let y = wa.position.y + (wa.size.height.saturating_sub(outer.height) / 2) as i32;
        let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
    }
    if maximized {
        let _ = win.maximize();
    }
}

/// Closes the app (Esc, or after a successful send in ShareX mode).
#[tauri::command]
fn quit(app: AppHandle) {
    remember_window(&app);
    app.exit(0);
}

#[derive(Serialize, Clone)]
struct SourceInfo {
    w: u32,
    h: u32,
    stem: String,
    rev: u64,
}

#[derive(Serialize)]
struct LaunchInfo {
    mode: Mode,
    output: Option<String>,
    source: Option<SourceInfo>,
    error: Option<String>,
}

impl AppState {
    fn set_source(&self, s: Source) -> SourceInfo {
        let rev = self.rev.fetch_add(1, Ordering::SeqCst) + 1;
        let info = SourceInfo { w: s.w, h: s.h, stem: s.stem.clone(), rev };
        *self.source.write().unwrap() = Some(Arc::new(s));
        info
    }

    fn current(&self) -> Option<SourceInfo> {
        let rev = self.rev.load(Ordering::SeqCst);
        self.source
            .read()
            .unwrap()
            .as_ref()
            .map(|s| SourceInfo { w: s.w, h: s.h, stem: s.stem.clone(), rev })
    }
}

/// Launch mode plus the launch source (waits for its decode). Safe to call again after a
/// page reload: it then reports the current source.
#[tauri::command]
async fn get_launch(state: State<'_, AppState>) -> Result<LaunchInfo, String> {
    let pending = state.startup.lock().unwrap().take();
    let (source, error) = match pending {
        Some(job) => match job.await {
            Ok(Ok(s)) => (Some(state.set_source(s)), None),
            Ok(Err(e)) => (None, Some(e)),
            Err(e) => (None, Some(e.to_string())),
        },
        None => (state.current(), None),
    };
    Ok(LaunchInfo {
        mode: state.launch.mode(),
        output: state.launch.output.as_ref().map(|p| p.display().to_string()),
        source,
        error,
    })
}

/// Replaces the source with a file (OPEN dialog or drag-and-drop).
#[tauri::command]
async fn load_path(path: String, state: State<'_, AppState>) -> Result<SourceInfo, String> {
    let s = async_runtime::spawn_blocking(move || source::load_path(std::path::Path::new(&path)))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    Ok(state.set_source(s))
}

/// Replaces the source with the clipboard image (Ctrl+V or PASTE).
#[tauri::command]
async fn paste_clipboard(state: State<'_, AppState>) -> Result<SourceInfo, String> {
    let s = async_runtime::spawn_blocking(source::read_clipboard)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    Ok(state.set_source(s))
}

/// Lets the UI write to the app's stdout; used to log timings during development.
#[tauri::command]
fn ui_log(message: String) {
    println!("[ui] {message}");
}

/// `ripper://` handler (design D4). `GET /source.rgba` returns the raw RGBA8 bytes of the
/// current source with its size in `X-Width` / `X-Height`; no image decoding in the WebView.
fn serve(app: &tauri::AppHandle, path: &str, query: Option<&str>) -> Response<Vec<u8>> {
    let base = Response::builder()
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Expose-Headers", "X-Width, X-Height")
        .header("Cache-Control", "no-store");
    if path.starts_with("/atlas/") {
        let state = app.state::<AppState>();
        return match atlas_cmd::serve_thumbnail(&state, path, query) {
            Some(png) => base.status(StatusCode::OK).header("Content-Type", "image/png").body(png).unwrap(),
            None => base.status(StatusCode::NOT_FOUND).body(Vec::new()).unwrap(),
        };
    }
    if path != "/source.rgba" {
        return base.status(StatusCode::NOT_FOUND).body(Vec::new()).unwrap();
    }
    let state = app.state::<AppState>();
    let src = state.source.read().unwrap().clone();
    match src {
        Some(s) => base
            .status(StatusCode::OK)
            .header("Content-Type", "application/octet-stream")
            .header("X-Width", s.w.to_string())
            .header("X-Height", s.h.to_string())
            .body(s.rgba.clone())
            .unwrap(),
        None => base.status(StatusCode::NO_CONTENT).body(Vec::new()).unwrap(),
    }
}

fn main() {
    let launch = launch::parse(std::env::args_os().skip(1));

    // Decode the launch file in parallel with WebView start-up.
    let startup = launch.source.clone().map(|p| {
        async_runtime::spawn_blocking(move || source::load_path(&p).map_err(|e| e.to_string()))
    });

    let settings_path = settings::default_path();
    let (settings, settings_warning) = settings::load(&settings_path);
    let (window_size, maximized) = (settings.window, settings.maximized);

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState {
            launch,
            source: RwLock::new(None),
            rev: AtomicU64::new(0),
            startup: Mutex::new(startup),
            settings: Mutex::new(settings),
            settings_path,
            settings_warning: Mutex::new(settings_warning),
            update: Mutex::new(None),
            store: Store::new(atlas_store::default_root()),
            held: Mutex::new(std::collections::HashSet::new()),
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                remember_window(window.app_handle());
            }
        })
        .setup(move |app| {
            if let Some(win) = app.get_webview_window("main") {
                place_window(&win, window_size, maximized);
            }
            // The UI shows the window after its first paint. If the page fails to load,
            // show it anyway so a ShareX task is never left waiting on an invisible window.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(3));
                if let Some(w) = handle.get_webview_window("main") {
                    if !w.is_visible().unwrap_or(true) {
                        let _ = w.show();
                    }
                }
            });
            update::spawn_check(app.handle());
            Ok(())
        })
        .register_asynchronous_uri_scheme_protocol("ripper", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            let path = request.uri().path().to_string();
            let query = request.uri().query().map(str::to_string);
            std::thread::spawn(move || responder.respond(serve(&app, &path, query.as_deref())));
        })
        .invoke_handler(tauri::generate_handler![
            get_launch,
            load_path,
            paste_clipboard,
            ui_log,
            get_settings,
            set_settings,
            export,
            quit,
            update::get_update,
            update::install_update,
            atlas_cmd::atlas_get,
            atlas_cmd::atlas_name,
            atlas_cmd::atlas_rename,
            atlas_cmd::atlas_revision,
            atlas_cmd::atlas_new,
            atlas_cmd::atlas_open,
            atlas_cmd::atlas_save,
            atlas_cmd::atlas_save_as,
            atlas_cmd::atlas_edit,
            atlas_cmd::atlas_apply_steps,
            atlas_cmd::atlas_load_guides,
            atlas_cmd::atlas_export
        ])
        .run(tauri::generate_context!())
        .expect("error while running ripper");
}
