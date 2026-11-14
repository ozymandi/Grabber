# app-distribution Specification

## Purpose

Sets how ripper is installed on a Windows machine and how it keeps itself current: a per-user installer, a signed update fetched from the project's public releases, and the rules that keep updates out of the ShareX capture flow.

## Requirements

### Requirement: Per-user installer
The release SHALL include a Windows setup program. It SHALL install ripper for the current user only, into `%LOCALAPPDATA%\ripper`, without administrator rights or a UAC prompt. It SHALL add a Start menu entry and register an uninstaller in Windows "Installed apps".

#### Scenario: Fresh install
- **WHEN** a user without administrator rights runs `ripper_1.0.0_x64-setup.exe` and accepts the defaults
- **THEN** `%LOCALAPPDATA%\ripper\ripper.exe` exists, a Start menu entry starts it, and no UAC prompt appeared

#### Scenario: Uninstall
- **WHEN** the user removes ripper from Windows "Installed apps"
- **THEN** the program folder and the Start menu entry are gone

### Requirement: Settings survive install, update and uninstall
Installing, updating or uninstalling SHALL NOT change or delete the user's settings file `%APPDATA%\ripper\config.toml`. That includes the output folders, the aspect defaults and the window size.

#### Scenario: Update keeps settings
- **WHEN** ripper 1.0.0 with both output folders set is updated to a newer version
- **THEN** the new version starts with the same folders, toggles, aspect defaults and window size

#### Scenario: Reinstall after uninstall
- **WHEN** the user uninstalls ripper, including the option to delete app data, and installs it again
- **THEN** the previous `config.toml` is still in place and is used

### Requirement: Update check only in standalone mode
In standalone mode the app SHALL check once per launch, in the background, whether a newer release exists. The check SHALL NOT delay the window or image loading. In ShareX mode the app SHALL NOT check for updates or make any network request.

#### Scenario: Standalone launch
- **WHEN** the app is started without arguments and the network is available
- **THEN** the window appears as fast as without the check, and the check completes in the background

#### Scenario: ShareX launch
- **WHEN** ShareX starts the app with `"$input" "$output"`
- **THEN** the app makes no network request at any point, and its exit behaviour is unchanged

### Requirement: Failed check is silent
If the update check fails, the app SHALL behave as if no update exists and show nothing. Causes include no network, a timeout, an unreachable or malformed release file, or a rejected signature.

#### Scenario: Offline
- **WHEN** the app starts in standalone mode with no network
- **THEN** no update button appears and the status row shows its normal startup message

### Requirement: Update availability is shown
When a newer version exists, the status row SHALL show `Update <version> available` once. An update button SHALL then appear in the status actions, to the left of Help, and stay visible for the rest of the session even after other status messages replace that text. When no newer version exists, the button SHALL NOT be shown.

#### Scenario: Newer version published
- **WHEN** ripper 1.0.0 starts in standalone mode and release 2.0.0 is published
- **THEN** the status row reads `Update 2.0.0 available` and the update button is visible next to Help

#### Scenario: Message replaced, button stays
- **WHEN** an update is available and the user then saves quads with Ctrl+S
- **THEN** the status row shows the save result and the update button is still visible

#### Scenario: Up to date
- **WHEN** the installed version equals the latest release
- **THEN** no update button is shown

### Requirement: Update installs only on the user's click
An update SHALL be downloaded and installed only after the user clicks the update button. Once clicked, the app SHALL download the new setup, verify its signature, install it without installer windows or prompts, and restart as the new version. The restarted app SHALL start in standalone mode with the arguments it was started with, and SHALL NOT restore quads.

#### Scenario: Install update
- **WHEN** the user clicks the update button and has no unsaved quads
- **THEN** the status row shows the download progress, the app closes, installs 2.0.0 quietly and starts again as 2.0.0

#### Scenario: No click, no install
- **WHEN** an update is available and the user closes the app without clicking the button
- **THEN** nothing is downloaded or installed, and the next launch offers the update again

### Requirement: Unsaved quads are not lost to an update
A quad is unsaved if it was created or changed after it was last exported, either by Send as the active quad or by Save All. If any quad is unsaved, clicking the update button SHALL NOT start the update right away. The app SHALL first warn, naming how many quads are unsaved, and SHALL start the update only on a second, confirming click.

#### Scenario: Unsaved quads
- **WHEN** the image has two quads edited after the last save and the user clicks the update button
- **THEN** nothing is downloaded, and the status row warns that 2 quads are unsaved and that clicking again discards them and updates

#### Scenario: Confirming click
- **WHEN** the warning is shown and the user clicks the update button again
- **THEN** the update starts

#### Scenario: Saved quads
- **WHEN** every quad was saved with Ctrl+S and none changed since
- **THEN** the first click starts the update without a warning

### Requirement: Only signed updates are installed
The app SHALL install an update only if its signature verifies against the public key built into the app. A download whose signature is missing or does not match SHALL be discarded. The installed version SHALL stay unchanged.

#### Scenario: Tampered setup
- **WHEN** the downloaded setup does not match its published signature
- **THEN** nothing is installed, the app stays open as the current version, and the status row reports that the update could not be verified

### Requirement: Failed update leaves the app usable
If downloading or installing fails after the click, the app SHALL stay open as the current version with its image and quads intact. The status row SHALL report the failure, and the update button SHALL stay so the user can retry.

#### Scenario: Download interrupted
- **WHEN** the network drops while the update is downloading
- **THEN** the app stays open with its quads, the status row reports that the download failed, and the update button is still visible
