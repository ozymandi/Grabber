//! Self-update (spec app-distribution, design D3). Runs entirely on the Rust side: the page
//! gets no updater permission, only the events below and two commands.
//!
//! Events: `update-available { version }`, `update-progress { done, total }`,
//! `update-installing`.

use crate::{remember_window, AppState};
use ripper_core::update_policy;
use serde::Serialize;
use std::time::Duration;
use tauri::{async_runtime, AppHandle, Emitter, Manager, State};
use tauri_plugin_updater::{Error, UpdaterExt};

/// Give up on the release server after this long; a failed check is silent.
const CHECK_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Serialize, Clone)]
struct Available {
    version: String,
}

#[derive(Serialize, Clone)]
struct Progress {
    done: u64,
    total: Option<u64>,
}

fn log(message: &str) {
    println!("[update] {message}");
}

/// Starts the background check, but only in standalone mode. It waits for the window to
/// be shown so it never competes with start-up.
pub fn spawn_check(app: &AppHandle) {
    let state = app.state::<AppState>();
    if !update_policy::may_check(&state.launch.mode()) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        for _ in 0..100 {
            let shown = app
                .get_webview_window("main")
                .and_then(|w| w.is_visible().ok())
                .unwrap_or(false);
            if shown {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        async_runtime::block_on(check(&app));
    });
}

async fn check(app: &AppHandle) {
    let handle = app.clone();
    let updater = app
        .updater_builder()
        .timeout(CHECK_TIMEOUT)
        // The installer ends this process; keep the window size like a normal close does.
        .on_before_exit(move || remember_window(&handle))
        .build();
    let found = match updater {
        Ok(u) => u.check().await,
        Err(e) => Err(e),
    };
    match found {
        Ok(Some(update)) => {
            let version = update.version.clone();
            log(&format!("{version} available"));
            *app.state::<AppState>().update.lock().unwrap() = Some(update);
            let _ = app.emit("update-available", Available { version });
        }
        Ok(None) => log("up to date"),
        Err(e) => log(&format!("check failed: {e}")),
    }
}

/// The version found by the background check, if any. The page asks once on load in case
/// `update-available` fired before it was listening.
#[tauri::command]
pub fn get_update(state: State<'_, AppState>) -> Option<String> {
    state.update.lock().unwrap().as_ref().map(|u| u.version.clone())
}

fn failure_text(e: &Error) -> String {
    match e {
        Error::Minisign(_)
        | Error::Base64(_)
        | Error::SignatureUtf8(_)
        | Error::SignedVersionMismatch { .. }
        | Error::MissingSignedVersion => "Update could not be verified, nothing was installed".into(),
        Error::Reqwest(_) | Error::Network(_) | Error::Io(_) => format!("Update download failed: {e}"),
        _ => format!("Update failed: {e}"),
    }
}

/// Downloads, verifies and installs the stored update. On success the installer takes over
/// and this process exits, so the call only returns on failure; the update stays stored so
/// the user can retry.
#[tauri::command]
pub async fn install_update(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let update = state.update.lock().unwrap().clone();
    update_policy::may_install(&state.launch.mode(), update.is_some()).map_err(|r| r.text().to_string())?;
    let update = update.expect("checked by may_install");
    let mut done: u64 = 0;
    let progress_app = app.clone();
    let result = update
        .download_and_install(
            move |chunk, total| {
                done += chunk as u64;
                let _ = progress_app.emit("update-progress", Progress { done, total });
            },
            || {
                let _ = app.emit("update-installing", ());
            },
        )
        .await;
    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            log(&format!("install failed: {e}"));
            Err(failure_text(&e))
        }
    }
}
