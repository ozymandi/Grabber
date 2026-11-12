# ripper

Grab flat surfaces from any image and turn them into straight, frontal textures. Mark the four corners of a sign, a wall, a floor or a label, and ripper removes the perspective. It is a modern take on the old ShoeBox texture ripper.

It works on its own, and it plugs into **ShareX**: capture a region with a hotkey, mark a quad, press Enter, and ShareX carries on with the flattened texture.

## Features

- Several quads per image. Drag any corner to adjust it; live preview of the active quad.
- Perspective-correct unwarp with adaptive supersampling, so shrunk areas stay clean, not jagged.
- Output size from the quad's edges, or locked to 1:1, 4:3, 2:1 or a custom ratio. Optional power-of-two sizes (256, 512, 1024...).
- **Enter** sends the active quad to the clipboard (with transparency), to ShareX and to your output folders.
- **Ctrl+S** saves every quad.
- Two output folders, for example a local folder and a Google Drive folder. Each can be switched on or off. Existing files are never overwritten.
- Input from ShareX, the clipboard (Ctrl+V), drag and drop, or Open: PNG, JPEG, BMP, WebP.

## Requirements

- Windows 10 or 11 (64-bit).
- Microsoft Edge WebView2 Runtime. It is part of Windows 11; on Windows 10 install it from Microsoft if it is missing.
- ShareX, for the capture workflow. These steps were written against ShareX 22.4.

## Install

There is no prebuilt download yet, so build ripper from source. It takes a few minutes the first time.

1. Install the build tools, once:
   - [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the **Desktop development with C++** workload. A full Visual Studio with that workload also works.
   - Rust, via [rustup](https://rustup.rs/) or `winget install --id Rustlang.Rustup --exact`.
   - The Tauri CLI: `cargo install tauri-cli --locked`.

2. Build:

   ```bash
   git clone https://github.com/ozymandi/Grabber.git
   cd Grabber/src-tauri
   cargo tauri build
   ```

   The result is a single portable file: `src-tauri\target\release\ripper.exe`.

3. Copy it to a permanent place, so later rebuilds do not touch the copy ShareX uses:

   ```powershell
   New-Item -ItemType Directory -Force "$env:LOCALAPPDATA\Programs\ripper"
   Copy-Item target\release\ripper.exe "$env:LOCALAPPDATA\Programs\ripper\ripper.exe"
   ```

   The rest of this guide assumes `%LOCALAPPDATA%\Programs\ripper\ripper.exe`.

4. Start it once on its own, open the **Output** panel and choose your folders (see [Output folders](#output-folders)).

## Connect to ShareX

Give ripper its own hotkey. Your existing capture hotkeys keep working exactly as before.

1. In ShareX open **Hotkey settings** and press **Add**.
2. In the new row:
   - set the task to **Capture region**;
   - click the hotkey field and press the combination you want, for example **Ctrl+Shift+F11**.
3. Click the row's **Edit task settings...** (gear) button.
4. **Task** page:
   - **Description:** `ripper`.
   - Tick **Override after capture tasks**. In **After capture tasks** tick **Save image to file** and **Perform actions**, and untick everything else, in particular **Upload image to host**. ripper already puts the result on the clipboard.
   - Tick **Override after upload tasks** and leave all of them unticked.
5. **Actions** page:
   - Tick **Override actions**, then **Add action**:

     | Field | Value |
     |---|---|
     | Name | `ripper` |
     | File path | `%LOCALAPPDATA%\Programs\ripper\ripper.exe` (use **Browse...**) |
     | Arguments | `"$input" "$output"` |
     | Output file name extension | `tex.png` |
     | Extensions | leave empty |
     | Hidden window | off |
     | Delete input file | off |

   - Make sure the new action is ticked (active).
6. Close the dialogs. ShareX saves the settings.

**Why `tex.png` and not `png`.** ShareX builds `$output` from the captured file's name plus this extension. With `png` the output would be the capture itself, and the original screenshot would be overwritten. With `tex.png` you keep `capture.png` and get `capture.tex.png` next to it.

### What happens on capture

```
Ctrl+Shift+F11 -> select a region -> ShareX saves capture.png -> ripper opens
   Enter -> texture written to capture.tex.png, clipboard and output folders
            -> ripper closes -> ShareX continues with capture.tex.png
   Esc   -> ripper closes, nothing written -> ShareX continues with capture.png
```

ShareX waits while ripper is open, so finish (Enter) or cancel (Esc) before the next capture is processed.

## Using ripper

1. **Mark a quad:** click the four corners of a flat surface, top-left first, then around it. The first corner becomes the texture's top-left; it is shown with a ring.
2. **Adjust:** drag any corner. Click a quad, or its row in **Quads**, to make it active. A red quad crosses itself or is too thin, and will not export.
3. **Size:** pick **EDGES**, **1:1**, **4:3**, **2:1** or **CUSTOM** in **Aspect**. Locked ratios keep the edge-based width and set the height. **Power of two** snaps both sides.
4. **Export:** press **Enter** (SEND ACTIVE), **Ctrl+S** (SAVE ALL) or **COPY**.

| Key | Action |
|---|---|
| Enter | Send the active quad (clipboard, ShareX, folders) |
| Ctrl+S | Save all quads to the folders |
| Ctrl+V | Paste an image from the clipboard |
| Esc | Cancel the unfinished quad; otherwise close without writing |
| Delete | Remove the active quad |
| Tab / Shift+Tab | Next / previous quad |
| Mouse wheel | Zoom at the cursor |
| Space + drag, or middle drag | Pan |
| F | Fit the image to the window |

The **?** icon at the bottom right opens the same help inside the app.

### Output folders

The **Output** panel has two slots, **Local folder** and **Google Drive folder**. Each has its own checkbox and **Browse** button, and both receive the same files. For Google Drive, pick a folder on the drive that Google Drive for desktop mounts, for example `G:\My Drive\Textures`. No sign-in is needed inside ripper.

- Files are named `<source>_q<N>.png`, for example `capture_q1.png`. Pasted images use `clipboard_<date>-<time>`.
- An existing name is never overwritten: ripper writes `capture_q1_2.png`, `capture_q1_3.png` and so on.
- If a folder cannot be written, for example because the drive is not mounted, the other outputs still go through. A red status names the folder, and in ShareX mode the window stays open so you see it.

Settings are stored in `%APPDATA%\ripper\config.toml`.

## Troubleshooting

- **The hotkey does nothing.** Another program may own the combination. Choose a different one in **Hotkey settings**.
- **ShareX continues with the original capture after Enter.** Check that **Output file name extension** is `tex.png` and the action is ticked. A red status in ripper means a write failed; read it before closing.
- **Nothing happens after the capture.** Both **Save image to file** and **Perform actions** must be ticked; actions only run on a saved file.
- **The window does not appear.** Install or repair the WebView2 Runtime.
- **The clipboard image has no transparency in some app.** ripper puts both PNG and bitmap formats on the clipboard. Apps that only read the bitmap drop the alpha.

## Uninstall

1. ShareX **Hotkey settings**: select the ripper row and press **Remove**.
2. Delete `%LOCALAPPDATA%\Programs\ripper`, `%APPDATA%\ripper` (settings) and `%LOCALAPPDATA%\app.ripper.texture` (WebView2 data).

## Development

| Path | Contents |
|---|---|
| `src-tauri/` | Rust: geometry, warp, export, settings, Tauri commands |
| `ui/` | Plain HTML, CSS and JS, no bundler. The look follows the O'Tool design system |
| `openspec/` | Specs and the archived change history |

```bash
cd src-tauri
cargo test                       # unit and integration tests
cargo test --release --test warp_quality -- --ignored --nocapture timing
cargo tauri dev                  # run with the dev profile
```

```bash
node ui/js/geom.test.mjs         # JS preview geometry matches the Rust export geometry
```
