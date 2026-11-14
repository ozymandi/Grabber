# Tasks

## 1. Signing key, version and installer

- [x] 1.1 The designer generates the signing key and sets the variables (D7). The developer gives the commands and never sees the password.
  - Run `cargo tauri signer generate -w %USERPROFILE%\.tauri\ripper.key`, with a password.
  - Set the user variables `TAURI_SIGNING_PRIVATE_KEY` (the path) and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
  - Back up the key and the password in a password manager.

  Verify in a new shell that `ripper.key` and `ripper.key.pub` exist and that both variables are set. Check only that they are set, without printing the values.
- [x] 1.2 Set `version = "1.0.0"` in `src-tauri/Cargo.toml` and remove `version` from `tauri.conf.json` (D2). Verify that `cargo tauri build` names the bundle `ripper_1.0.0_x64-setup.exe`.
- [x] 1.3 Turn on bundling in `tauri.conf.json` (D1):
  - `active`;
  - `targets: ["nsis"]`;
  - `createUpdaterArtifacts`;
  - `windows.nsis.installMode: "currentUser"`;
  - the public key in `plugins.updater.pubkey`.

  Verify that `cargo tauri build` writes `ripper_1.0.0_x64-setup.exe` and `ripper_1.0.0_x64-setup.exe.sig` to `C:\cargo-target\release\bundle\nsis\`.
- [x] 1.4 Check the installer against the spec scenarios "Fresh install", "Uninstall" and "Reinstall after uninstall":
  1. Install as the current user. There must be no UAC prompt, the app goes to `%LOCALAPPDATA%\ripper\ripper.exe`, and the Start menu entry starts it.
  2. Uninstall with "delete app data" ticked. The program folder and the shortcut are gone, and `%APPDATA%\ripper\config.toml` is unchanged (compare its hash before and after).

  Leave the machine uninstalled afterwards, so migration in group 6 starts clean.

## 2. Updater on the Rust side

- [x] 2.1 Add `tauri-plugin-updater` 2.x and register it in `main.rs`. Add `plugins.updater` to `tauri.conf.json`: the endpoint from D3 and `windows.installMode: "quiet"`. Give the page no updater permission. Verify that `cargo build` passes and that `capabilities/default.json` is unchanged.
- [x] 2.2 Write `update.rs` with the background check (D3):
  - spawned only in standalone mode, after the window is shown;
  - a 10 s timeout;
  - stores the `Update` in `AppState`;
  - emits `update-available`;
  - drops every error, logging it at debug level.

  Add the command `get_update`. Put the mode decision in a pure function. Verify with unit tests that ShareX mode never starts a check and standalone mode always does.
- [x] 2.3 Add the command `install_update` (D3):
  - refuses in ShareX mode and when no update is stored;
  - emits `update-progress` and `update-installing`;
  - on error, returns the message and keeps the stored update for a retry.

  Verify with unit tests for both refusals, and that `cargo test` passes in full: the 49 existing tests plus the new ones.

## 3. Update UI

- [x] 3.1 Add the `dirty` flag per quad to `state.js` (D6). It is set on create and on corner, aspect or power-of-two changes. It is cleared after Send (active quad) or Save All (each exported quad) whenever that quad reached at least one destination. Add `unsavedCount()`. Verify in the `?demo` page through CDP:
  - create two quads: the count is 2;
  - Ctrl+S: 0;
  - move a corner: 1;
  - Enter on that quad: 0;
  - Esc and close behave as in 0.1.0.
- [x] 3.2 Find an update glyph in the O'Tool sources (O'Frame `main.css` and assets, O'Zometrix `ui.html`). If none fits, ask the designer for an SVG and wait, with no Unicode or PNG stand-in. Add it to `icons.js`, and add `#updateBtn` (hidden, `icon-btn`) before `#helpBtn`. Verify by measurement in the `?demo` page that the button is the same size as Help, has the same hover and focus states, keeps the status row 16 px high, and has a `title` and an `aria-label`.
- [x] 3.3 Write `update.js` (D4):
  - read `get_update` on load and listen for `update-available`;
  - show the button and the one-time `Update <version> available`;
  - the click flow, with the armed confirmation and disarming when quads change or the status text is replaced;
  - progress `Downloading update <version>: NN%`;
  - the failure message, with the button kept.

  Add stub events to `dev.js` for the demo page. Verify in `?demo` the spec scenarios "Newer version published", "Message replaced, button stays", "Up to date", "Unsaved quads", "Confirming click" and "Saved quads". Show the status wording to the designer for approval.

## 4. Update end to end (local test pair)

- [x] 4.1 Build a local test pair, kept out of the repository:
  - a throwaway signing key with no password, in the scratchpad;
  - a `--config` override JSON that points the endpoint at `http://127.0.0.1:<port>/latest.json`, with `dangerousInsecureTransportProtocol` and the throwaway public key;
  - builds `1.0.0-test` and `1.0.1-test`;
  - a small local HTTP server serving `latest.json` and the 1.0.1 setup, logging every request.

  Verify that the server serves both files and that the 1.0.1 signature is in `latest.json`.
- [x] 4.2 Install `1.0.0-test`.
  1. Start it standalone. The update button appears and the server logs one request.
  2. Start it in ShareX mode with `"<png>" "<out>.tex.png"`. The server logs no request, and Enter and Esc behave and exit as in 0.1.0.
  3. Compare startup times to visible window, 5 runs each, with the server up and down. Startup must stay within the 0.54–0.66 s measured for 0.1.0.

  Record the timings in `task.md`.
- [x] 4.3 Click update with no quads. Verify that:
  - the progress is shown;
  - the app closes and 1.0.1-test installs with no installer window or UAC prompt;
  - the app restarts by itself as 1.0.1-test in standalone mode;
  - `config.toml` is unchanged.

  If the app does not restart, add `/R` through `plugins.updater.windows.installerArgs` (D5) and repeat.
- [x] 4.4 Run the failure scenarios:
  - **Tampered setup:** serve a modified setup with the original `.sig`. Nothing installs, the app stays and the status reports that the update could not be verified.
  - **Download interrupted:** stop the server mid-download, for example with throttled serving. The app stays with its quads, the status reports the failure, and a retry after restarting the server works.
  - **Offline:** server down at start. No button and no message.
- [x] 4.5 Check that an update downloaded by the app does not trigger SmartScreen: no prompt during 4.3. Check that the downloaded setup has no `Zone.Identifier` stream. Record the result in `task.md`.
- [x] 4.6 Uninstall the test build, delete the throwaway key and the test bundles, and stop the server. Verify that `%LOCALAPPDATA%\ripper` is gone and `config.toml` is unchanged.

## 5. Release script and documentation

- [x] 5.1 Write `scripts/release.ps1` (D8):
  - preconditions: clean tree, tag not taken, signing variables set;
  - build;
  - collect the setup and `.sig`;
  - write `latest.json`;
  - confirm, then publish with `gh release create`.

  Add `-DryRun`, which stops before publishing. Verify with `-DryRun` that `latest.json` has `version` 1.0.0, a valid RFC 3339 `pub_date`, the exact `.sig` text and the asset URL from D8. Verify also that a missing variable stops the script before building.
- [x] 5.2 Update `README.md`:
  - **Install:** download `ripper_<version>_x64-setup.exe` from Releases, mention the SmartScreen "More info → Run anyway" step, and give the install path `%LOCALAPPDATA%\ripper`.
  - **Updates:** the button in the status row, standalone only.
  - **ShareX:** the action path becomes `%LOCALAPPDATA%\ripper\ripper.exe`.
  - **Building from source:** its own section, with the release script mentioned for maintainers.

  Verify that every path and label in the README matches the built installer and the app.

## 6. Release 1.0.0 and migration

- [x] 6.1 Run `scripts/release.ps1` for 1.0.0. Ask the designer for a yes before the publish step, because it is public. Verify that `https://github.com/ozymandi/Grabber/releases/latest/download/latest.json` downloads and names 1.0.0, and that the setup URL in it downloads.
- [x] 6.2 Migrate this machine (design: Migration Plan):
  1. install 1.0.0 from the release;
  2. back up `HotkeysConfig.json` as `*.before-1.0.0`;
  3. stop ShareX, repoint the ripper action to `%LOCALAPPDATA%\ripper\ripper.exe`, and restart ShareX with `-silent`.

  Verify Ctrl+Shift+F11 end to end: Enter writes `*.tex.png` and the original stays, Esc leaves the original. Only then delete `%LOCALAPPDATA%\Programs\ripper`.
- [x] 6.3 Start the installed 1.0.0 standalone against the real endpoint. Verify that no update button appears (spec "Up to date").
- [x] 6.4 Update `task.md` (status, timings, SmartScreen result, next step: atlas 2.0.0) and the project memory (install path, release process, ShareX path). Verify that both name the new path and the release script.

## Workflow follow-up

- The full update path on real releases is first exercised when 2.0.0 (atlas) is published. Re-check the spec scenario "Install update" then.
- Archive the change after the designer reviews the result.
