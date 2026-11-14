# Proposal

## Why

ripper 0.1.0 is installed by hand: build it from source, then copy `ripper.exe` into `%LOCALAPPDATA%\Programs\ripper`. Every fix means rebuilding and copying again on every machine. The next feature, the texture atlas (2.0.0), should reach the installed app as an update, not as another manual copy. So this change ships 1.0.0 as an installer and adds a signed self-update.

## What Changes

- **Installer.** Tauri's NSIS bundle `ripper_1.0.0_x64-setup.exe`:
  - installs for the current user into the stock path `%LOCALAPPDATA%\ripper`;
  - needs no administrator rights and shows no UAC prompt;
  - adds a Start menu entry and an uninstaller.

  Settings in `%APPDATA%\ripper\config.toml` survive install, update and uninstall.
- **Self-update** from GitHub Releases of the public repository `ozymandi/Grabber`:
  - When started standalone, the app checks `latest.json` once in the background. In ShareX mode it never checks and never touches the network.
  - When a newer version exists:
    - the status row says so once;
    - an icon button appears in the status actions, to the left of Help, and stays until the update is installed.
  - Clicking the button downloads the installer, verifies its signature, installs it quietly and restarts the app.
  - If there are unsaved quads, the app asks for confirmation first.
  - Nothing installs without that click.
- **Signed releases.** Every release is signed with a minisign key. The app holds the public key and rejects an update whose signature does not match.
- **Version 1.0.0**, kept only in `Cargo.toml`.
- **Release process.** Releases are built locally with `cargo tauri build` and published with `gh release create`: installer, `.sig` and `latest.json`.
- **Migration on this machine**, done once by hand:
  1. install 1.0.0;
  2. repoint the ShareX action to `%LOCALAPPDATA%\ripper\ripper.exe`;
  3. remove the old `Programs\ripper` copy.

  The installer does not touch ShareX settings.
- **README.** Installation becomes "download the setup from Releases". The ShareX section uses the new path. Building from source moves to its own section.

Out of scope:
- Authenticode code signing. The first install shows SmartScreen's "unknown publisher" warning.
- Automatic installation without a click.
- Release channels: beta, rollback.
- Building releases in CI.
- A private source repository.
- The texture atlas, which is a separate change, 2.0.0.

## Capabilities

### New Capabilities
- `app-distribution`: how ripper gets onto a machine and stays current. Covers:
  - the per-user installer and uninstaller, and that settings survive them;
  - the background update check and that ShareX mode never checks;
  - how an available update is shown;
  - installing an update after confirmation, with signature verification;
  - how a failed check, download or install is handled.

### Modified Capabilities
- (none: quad editing, unwarping and export behave as before; ShareX mode already exits straight after output, and this change only forbids adding network work to it, which `app-distribution` states)

## Impact

- **Code:**
  - `src-tauri/Cargo.toml`:
    - version 1.0.0;
    - new dependency `tauri-plugin-updater`.
  - `src-tauri/tauri.conf.json`:
    - remove `version`;
    - turn bundling on: NSIS only, `currentUser`, updater artifacts;
    - updater plugin config: public key, endpoint, quiet install.
  - `src-tauri/src/main.rs` registers the plugin. A new `src-tauri/src/update.rs` runs the check and the install on the Rust side, so the page never gets direct updater permissions.
  - A new `scripts/release.ps1` builds, writes `latest.json` and publishes the release.
  - UI:
    - `ui/index.html` gets the update button;
    - a new `ui/js/update.js` holds the check, the button and the confirmation;
    - `ui/js/state.js` tracks unsaved quads;
    - `ui/js/icons.js` gets the update icon.
- **New files outside the repo:** `%USERPROFILE%\.tauri\ripper.key`, the private signing key, protected with a password. The designer generates it and sets the user environment variables `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
- **GitHub:** first release `v1.0.0` on `ozymandi/Grabber`, with `latest.json`.
- **This machine:**
  - the ShareX action path changes (`HotkeysConfig.json`);
  - the old copy in `%LOCALAPPDATA%\Programs\ripper` is removed.
- **Docs:** `README.md` and `task.md`.
- **Design input:** the update icon must be an SVG from the O'Tool set. If none fits, the designer draws it.
