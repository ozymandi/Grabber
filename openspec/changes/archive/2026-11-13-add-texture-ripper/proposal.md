# Proposal

## Why

ShoeBox's texture ripper, which grabbed textures from any image and flattened them to a frontal view, no longer works: it is an abandoned Adobe AIR app. There is no fast tool left that goes straight from a ShareX screenshot to a straightened texture. Reviving the old binary would bring back the old UX and nothing more, so we build a new Windows tool. It plugs into ShareX's capture flow and uses the O'Tool interface language.

## What Changes

- New desktop app (working name **ripper**; product name to be decided), built with Rust + Tauri 2. The UI is plain HTML/CSS/JS rendered by WebView2.
- Load an image from a command-line path (ShareX action), the clipboard (Ctrl+V), drag-and-drop or an Open dialog.
- Place and edit **several quads** per image: drag the four corners, zoom and pan, select the active quad, delete quads.
- Live preview of the active quad flattened to a frontal view. The final export is a high-quality perspective unwarp in Rust.
- Output size from edge lengths by default. Per-quad aspect lock: presets and custom values. Optional snapping to power-of-two sizes.
- Output actions:
  - **Enter** sends the active quad to the clipboard, the ShareX `$output` file (when launched by ShareX) and the enabled output folders, then exits in ShareX mode.
  - **Ctrl+S** saves all quads to the enabled output folders.
  - **Esc** exits without output, so ShareX keeps the original capture.
- Two output folders, local and Google Drive (the Drive-for-desktop mount, e.g. `G:\`). Each is chosen in the app and can be turned on or off. They persist between runs. Existing files are never overwritten.
- The UI copies the O'Tool design system one to one: tokens and components from O'Frame's `css/main.css`, rules from O'Zometrix `DESIGN_SYSTEM.md`.

Out of scope for this change: atlas packing, seamless tiling, quads that share edges, automatic aspect estimation, uploading to Google Drive through its API, macOS and Linux builds.

## Capabilities

### New Capabilities
- `quad-editing`: loading a source image and creating, selecting, editing and removing perspective quads on it, with zoom and pan and a live preview of the active quad.
- `quad-unwarp`: turning a quad into a frontal texture. Covers the perspective mapping, output size and aspect rules, power-of-two snapping and sampling quality.
- `texture-export`: where results go. Covers the clipboard, the ShareX launch and `$output` contract, two toggleable output folders with persisted settings, file naming without overwrites, and handling an unavailable folder.

### Modified Capabilities
- (none: the project has no existing specs)

## Impact

- **New code** in `E:\Projects\Rust`, which so far holds only OpenSpec scaffolding:
  - a Tauri 2 crate in `src-tauri/`;
  - a static frontend in `ui/`;
  - git initialised for the project.
- **Toolchain:**
  - rustup with the `stable-x86_64-pc-windows-msvc` toolchain, which is not installed yet;
  - Tauri CLI through `cargo install tauri-cli`.
  - The MSVC build tools (VS 2026) and the WebView2 runtime (157) are already present. npm is not required.
- **Rust dependencies:** `tauri`, `tauri-plugin-dialog`, `arboard`, `image`, `rayon`, `serde`, `toml`, `dirs`, `chrono` (local time for pasted-image names).
- **External setup:** a ShareX 22.4 action:
  - arguments `"$input" "$output"`, output extension `tex.png` (ShareX builds `$output` from the input name plus this extension, so plain `png` would overwrite the original capture);
  - "Save image to file" and "Perform actions" turned on in After capture.
- **Design source:** O'Tool CSS and docs are read from `E:\Projects\Illustrator\Plugins\O'Frame` and `E:\Projects\O'Tool\O'Zometrix`. The originals are not modified.
