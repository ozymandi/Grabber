# Design

## Context

ripper 0.1.0 is a portable `ripper.exe`. Bundling is off (`"bundle": { "active": false }` in `tauri.conf.json`). The version `0.1.0` is written in both `Cargo.toml` and `tauri.conf.json`. There are no git tags or GitHub releases. The repository `ozymandi/Grabber` is public.

Facts that shape the design:
- **Build output.** `E:\Projects\.cargo\config.toml` sends build output to `C:\cargo-target`, because Google Drive mirrors `E:\Projects`. Bundles therefore land in `C:\cargo-target\release\bundle\nsis\`.
- **Install folder.** Tauri's NSIS template (`tauri-bundler 2.10.4`, `installer.nsi`) installs a `currentUser` app into `$LOCALAPPDATA\${PRODUCTNAME}`, which is `%LOCALAPPDATA%\ripper`.
- **Update mode.** With `/UPDATE` the template runs in update mode. In silent and passive runs it honours `/R` (relaunch).
- **Uninstall.** Its "delete app data" option removes `$APPDATA\${BUNDLEID}` and `$LOCALAPPDATA\${BUNDLEID}`, which are `app.ripper.texture`. Our settings live in `%APPDATA%\ripper`, so the option does not touch them.
- **Launch modes.** `launch.rs` decides the mode: a second argument means ShareX mode. The UI knows it as `state.mode`.
- **No "unsaved" state.** Quads are never marked as exported. Esc and close discard them without asking.
- **Status row.** `#statusDot`, `#status` and `.status-actions`, which holds only `#helpBtn` (`icon-btn info-btn`). Status text is replaced by every action.

Requirements are in `specs/app-distribution`.

## Goals / Non-Goals

**Goals:**
- A ShareX launch behaves exactly as in 0.1.0: same startup time, no network, same exit.
- The update path is controlled from Rust, so a page bug cannot trigger a network call in ShareX mode.
- One command builds and publishes a release, so the release process cannot drift.

**Non-Goals:**
- Delta updates or background downloads before the click.
- Showing release notes in the app.
- Supporting several installed copies side by side.

## Decisions

### D1. Bundle: NSIS only, per user, stock template

`tauri.conf.json`:
- `bundle.active: true`;
- `bundle.targets: ["nsis"]`;
- `bundle.createUpdaterArtifacts: true`;
- `bundle.windows.nsis.installMode: "currentUser"`.

The rest of the template stays at Tauri's defaults: WebView2 bootstrapper, Start menu entry, the uninstaller.

- *Alternative: MSI (WiX).* MSI is per-machine by default and needs admin rights, and its updates go through `msiexec`, which can prompt for UAC. Rejected.
- *Alternative: a custom NSIS template to keep `Programs\ripper`.* The designer rejected it: the template would have to be re-synced with every `tauri-bundler` release, because update mode depends on it.

### D2. One version source

Remove `version` from `tauri.conf.json`. Tauri then takes it from `Cargo.toml`, which becomes `1.0.0`. The release script reads the version from `Cargo.toml` too. The tag is `v<version>`.

### D3. Updater runs in Rust, the page only gets events

Add `tauri-plugin-updater` 2.x on the Rust side only. The page gets no `updater:*` permission.

A new module `update.rs`:
- **Background task.** `setup()` spawns it only when `launch.mode() == Standalone`. It waits until the window is shown, then calls `app.updater()?.check()` with a 10 s timeout. A newer version goes into `AppState.update: Mutex<Option<Update>>`, and the task emits `update-available { version }` to the page. Every error is dropped; it is logged through the existing `ui_log` path at debug level.
- **Command `install_update`.**
  - It returns an error in ShareX mode and when no update is stored.
  - Otherwise it calls `download_and_install`, emitting `update-progress { done, total }` per chunk and `update-installing` before the installer starts.
  - On Windows the plugin starts the setup and the process exits. Restarting is the installer's job (D5).
  - On error it returns a message, keeps the stored `Update` so a retry works, and the app stays as it is.
- **Command `get_update`.** Returns the version that is already known, or null. The page asks for it once on load, in case the event fired before the page was listening.

Endpoint: `https://github.com/ozymandi/Grabber/releases/latest/download/latest.json`.

`plugins.updater` in `tauri.conf.json` holds:
- the `pubkey`;
- the endpoint;
- `windows.installMode: "quiet"`, so there are no installer windows. This is safe because a per-user install needs no UAC;
- `requireSignedVersion: true`. Tauri CLI 2.12.4 writes the version into the signature's trusted comment, so a forged `latest.json` cannot pair a higher version number with an older, validly signed setup. 1.0.0 is the first signed release, so there are no older signatures without a version to re-sign.

- *Alternative: call `window.__TAURI__.updater.check()` from JS.* That is simpler, but then ShareX mode would be kept offline only by JS discipline. Rejected under the first goal.

### D4. Update UI

- **Button.** `index.html` gets `<button id="updateBtn" class="icon-btn" hidden>` inside `.status-actions`, before `#helpBtn`. It has the same size and states as Help, a `title` of "Update to <version>" and a matching `aria-label`.
- **Icon.** Added to `icons.js` as SVG. It must be an O'Tool glyph, searched for in O'Frame `main.css` and assets and in O'Zometrix `ui.html`. If none fits, the designer supplies one. No Unicode stand-in.
- **`update.js`:**
  - asks `get_update` on load and listens for `update-available`;
  - shows the button;
  - writes `Update <version> available` to the status once, as `info`.
- **On click:**
  - with no unsaved quads, it calls `install_update`;
  - otherwise it arms a confirmation and shows `N unsaved quad(s) will be lost. Click update again to install` as `error`. A second click while armed calls `install_update`.

  The confirmation disarms when quads change or when another status message replaces it, so a stale warning never turns a later click into a discard.
- **Progress.** `Downloading update <version>: NN%` as `info`. On failure the status shows the error as `error`, and the button stays.

Status wording is new UI copy for the designer to review during implementation.

### D5. Restart after install

`tauri-plugin-updater` 2.14.1 passes `/R` itself (`restart_after_install` defaults to true), followed by `/ARGS` and the arguments the app was started with. The quiet NSIS run in update mode then relaunches the app. Task 4.3 confirms this on a real install.

Install is refused in ShareX mode, so those arguments are at most one image path. The relaunched app therefore runs in standalone mode, and reopens that image if it was started with one. Quads are not carried over.

- *Alternative: `tauri-plugin-process` with `relaunch()`.* It cannot run after the plugin has exited the process for the installer. Not needed.

### D6. Unsaved quads

`state.js` gives each quad a `dirty` flag:
- set on create and on any corner, aspect or power-of-two change;
- cleared for the active quad after a Send, and for every exported quad after Save All, when the export wrote that quad to at least one destination (clipboard, ShareX output or a folder).

A partial failure, such as Drive being offline, still counts as saved, because the user already has the file. `unsavedCount()` counts the dirty quads. Esc and close keep their 0.1.0 behaviour; the flag is used only by the update button.

### D7. Signing key handling

- The designer generates the key with `cargo tauri signer generate -w %USERPROFILE%\.tauri\ripper.key`, with a password, and sets the user environment variables `TAURI_SIGNING_PRIVATE_KEY` (the path) and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
- The public key, the content of `ripper.key.pub`, goes into `tauri.conf.json`. It is public by design.
- The private key and the password never enter the repository, `E:\Projects` or `~/.claude`. The developer never sees the password.
- The release script fails early if either variable is missing.

### D8. Release script

`scripts/release.ps1`:
1. **Preconditions.** The working tree is clean, the tag `v<version>` does not exist yet, and both signing variables are set.
2. **Build.** `cargo tauri build`, from `src-tauri`.
3. **Collect.** Takes `ripper_<v>_x64-setup.exe` and its `.sig` from `C:\cargo-target\release\bundle\nsis\`.
4. **Write `latest.json`:**
   - `version`, `pub_date` (UTC, RFC 3339) and short `notes`;
   - `platforms["windows-x86_64"]`, with the signature text and the asset URL `https://github.com/ozymandi/Grabber/releases/download/v<v>/ripper_<v>_x64-setup.exe`.
5. **Publish.** `gh release create v<v>` with the three files, titled `ripper <v>`.

The script asks for confirmation before publishing, because publishing is public. Tauri does not write `latest.json` itself; `tauri-action` does, but that runs in CI, which is out of scope.

## Risks / Trade-offs

- **[Lost signing key]** Installed copies can no longer update. → Back up the key and the password in a password manager (D7). Recovery is a manual reinstall of a build with a new key.
- **[SmartScreen on first install]** The setup is not Authenticode-signed, so Windows shows "unknown publisher". → Documented in the README. Updates downloaded by the app carry no Mark-of-the-Web, so they are not expected to trigger it; task 4.5 checks this.
- **[Relaunch]** The installer may not relaunch the app on this machine. → Verified in task 4.3 (D5).
- **[GitHub unavailable or rate-limited]** The check fails. → The check is silent (spec), and the next launch retries.
- **[A broken release reaches the machine]** → Nothing installs without a click (variant A). A fixed release can be published over it.
- **[ShareX path changes]** Until the action is repointed, ShareX runs the old 0.1.0 copy. → The migration does the install, repoint and test in one sitting, and the old copy is deleted only after Ctrl+Shift+F11 works with the new path.

## Migration Plan

On this machine, once, after `v1.0.0` is published:
1. Run `ripper_1.0.0_x64-setup.exe`. Check `%LOCALAPPDATA%\ripper\ripper.exe` and the Start menu entry.
2. Stop ShareX.
3. In `C:\Users\ozyma\OneDrive\Documents\ShareX\HotkeysConfig.json`, change the ripper action's path from `%LOCALAPPDATA%\Programs\ripper\ripper.exe` to `%LOCALAPPDATA%\ripper\ripper.exe`. Keep the existing `*.before-ripper` backups, and add a new `*.before-1.0.0` copy before editing.
4. Start ShareX with `-silent`.
5. Run Ctrl+Shift+F11 end to end: Enter writes `*.tex.png`, and Esc leaves the original.
6. Delete `%LOCALAPPDATA%\Programs\ripper`.

**Rollback:** restore `HotkeysConfig.json` from `*.before-1.0.0`, uninstall 1.0.0, and copy a 0.1.0 build back to `Programs\ripper`. Settings are untouched in either direction.
