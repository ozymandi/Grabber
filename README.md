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
- **Atlas** (2.0): every texture you send can also land on an atlas, a grid of square power-of-two sectors in the UDIM layout. Arrange the tiles and export one sector, a sheet, or a numbered set `name.1001.png`, `name.1002.png`...
- **Fine work** (2.1): UV-layout guides under the tiles, free rotation about a movable pivot, edge fill on export and undo.
- **Shapes** (2.3): cut a circle, a regular N-gon or a Bezier path out of a quad, with transparency outside; undo in RIP.

## Requirements

- Windows 10 or 11 (64-bit).
- Microsoft Edge WebView2 Runtime. It is part of Windows 11; on Windows 10 the installer downloads it if it is missing.
- ShareX, for the capture workflow. These steps were written against ShareX 22.4.

## Install

1. Download `ripper_<version>_x64-setup.exe` from the [latest release](https://github.com/ozymandi/Grabber/releases/latest).
2. Run it. ripper installs for your Windows user only, into `%LOCALAPPDATA%\ripper`, with no administrator rights.
   - The installer is not code-signed, so Windows SmartScreen may say it "protected your PC". Click **More info**, then **Run anyway**.
   - On Windows 10 the installer fetches the WebView2 Runtime if it is missing.
3. Start **ripper** from the Start menu, open the **Output** panel and choose your folders (see [Output folders](#output-folders)).

The rest of this guide assumes `%LOCALAPPDATA%\ripper\ripper.exe`.

### Updates

When you start ripper on its own (not from ShareX), it checks for a newer release in the background. If there is one, the status row says so and an update icon appears next to **?**. Click it to download, verify and install the update; ripper restarts as the new version. If quads are unsaved, the first click warns you and a second click confirms.

Nothing is downloaded until you click, and a ripper started by ShareX never checks for updates, so captures are never slowed down. Updates keep the same install folder, so the ShareX action keeps working.

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
     | File path | `%LOCALAPPDATA%\ripper\ripper.exe` (use **Browse...**) |
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
4. **Shape:** the icon beside **Quads**, or **Shift+S**, sets what is cut out of the active quad. The quad stays the flat plane, and the shape is a mask on it: outside it the texture is transparent. New quads take the last shape.
   - **Rectangle:** the whole quad, as before.
   - **Circle:** the ellipse inscribed in the quad. Put the quad's corners on the square around a round sign.
   - **N-gon:** a regular polygon with 3 to 12 sides that touches all four sides of the quad. 8 fits a STOP sign.
   - **Path:** a closed Bezier path, edited as with Illustrator's Pen. It starts from the current shape.
     - Click a segment to add an anchor; click an anchor to remove it.
     - **Ctrl+drag** moves an anchor or a handle.
     - **Alt+drag** from an anchor pulls out handles; **Alt+click** makes it a sharp corner again; **Alt+drag** on a handle breaks it from its partner.
     - **Shift** keeps a drag in 45-degree steps. **P** draws the path anew: click for a sharp anchor, click and drag for a smooth one, click the first anchor to close.
   - Every shape is outlined with marching ants. The quad's edges and corner handles show only on the active quad.
5. **Export:** press **Enter** (SEND ACTIVE), **Ctrl+S** (SAVE ALL) or **COPY**. **Ctrl+Z** undoes quad edits; a new image clears the history.

| Key | Action |
|---|---|
| Enter | Send the active quad (clipboard, ShareX, folders) |
| Ctrl+S | Save all quads to the folders |
| Ctrl+V | Paste an image from the clipboard |
| Esc | Cancel the unfinished quad or Pen drawing; otherwise close without writing |
| Delete | Remove the active quad |
| Tab / Shift+Tab | Next / previous quad |
| Mouse wheel | Zoom at the cursor |
| Space + drag, or middle drag | Pan |
| F | Fit the image to the window |
| Shift+S | Next shape: rectangle, circle, N-gon, path |
| P | Draw the active path anew with the Pen |
| Ctrl+Z / Ctrl+Shift+Z, Ctrl+Y | Undo / redo a quad edit |

The **?** icon at the bottom right opens the same help inside the app.

### Output folders

The **Output** panel has two slots, **Local folder** and **Google Drive folder**. Each has its own checkbox and **Browse** button, and both receive the same files. For Google Drive, pick a folder on the drive that Google Drive for desktop mounts, for example `G:\My Drive\Textures`. No sign-in is needed inside ripper.

- Files are named `<source>_q<N>.png`, for example `capture_q1.png`. Pasted images use `clipboard_<date>-<time>`.
- An existing name is never overwritten: ripper writes `capture_q1_2.png`, `capture_q1_3.png` and so on.
- If a folder cannot be written, for example because the drive is not mounted, the other outputs still go through. A red status names the folder, and in ShareX mode the window stays open so you see it.

Settings are stored in `%APPDATA%\ripper\config.toml`.

## Atlas

The **RIP | ATLAS** switch at the bottom left of the canvas changes the mode. RIP is the quad editor described above; ATLAS arranges the textures you collected. A launch from ShareX always opens in RIP.

### Collecting tiles

Tick **Atlas** in the **Output** panel. From then on **Enter** (also from ShareX) and **Ctrl+S** add the texture as a tile to the current workspace, next to the clipboard and folders. The field under the checkbox names the workspace.

A new tile goes into free space of the sector you selected last, keeping the workspace padding; tiles already there never move. A tile larger than a sector is scaled down to fit, and its full-size original is kept. A tile that fits nowhere waits on the canvas beside the grid. Anything outside the sectors is never exported.

### Workspace

A workspace is a `.ripper` file: the sectors, where each tile sits, every original at full size with every picture in its History, the guides, and the guide and fill settings. Files saved by 2.4 need ripper 2.4 or later (older versions refuse them rather than lose the History); 2.4 opens files from 2.0 to 2.3 as they were. **NEW**, **OPEN**, **SAVE** and **SAVE AS** are in the **Workspace** panel. The workspace you opened last stays current, also for ShareX captures.

Every change is kept at once in `%APPDATA%\ripper\workspaces`, so closing the window loses nothing; a blue dot next to **Workspace** means the `.ripper` file is not saved yet.

All sectors of a workspace have one size: 512, 1024, 2048, 4096 or 8192. It can be changed only while the workspace is empty. **Padding, px** is the gap kept between tiles that are placed automatically.

### Sectors and tiles

- Click a sector to select it. **+** on its free sides adds a neighbour, flush, like UDIM tiles. **REMOVE** in the **Sector** panel deletes an empty sector.
- Click a tile to select it, **Shift+click** to add more. Drag to move; a tile may lie across sector borders.
- Drag on empty canvas, in a sector or beside it, to draw a frame: it selects every tile it touches, by the tile's turned outline. **Shift** adds to the selection, **Esc** cancels the frame. A plain click on empty canvas still selects the sector.
- While you drag, the edges of the selection snap to sector borders, to the edges of other tiles and to the grid lines on screen. Borders and tiles snap twice: flush and at the workspace padding, so automatic and manual gaps match. A red line shows what the selection snapped to. Hold **Ctrl** to move without snapping.
- Drag a corner handle to scale a single tile; the size shows in the status row. Scaling always starts from the original, so making a tile small and large again loses nothing.
- **R** rotates the selection a quarter turn clockwise, **Shift+R** counter-clockwise. Quarter turns never resample the pixels. **Delete** removes it. **Esc** clears the selection.
- Drag just outside a corner of a single selected tile to rotate it freely; hold **Shift** for 15° steps. The status row shows the angle.
- Every tile turns about its own **pivot**, the crosshair in its centre. **Alt+drag** moves it (it snaps to the corners, edge midpoints and centre), **Alt+double-click** puts it back in the centre, and **Alt+drag** on a corner handle scales about it. The pivot is saved with the tile.
- **Ctrl+Z** undoes your own changes in this window (up to 100 steps), **Ctrl+Shift+Z** or **Ctrl+Y** redoes them. Tiles that arrive from ShareX or RIP are never undone, and settings such as padding, opacity or fill are not part of the history. If something was changed elsewhere in the meantime, the part that can no longer be undone is skipped and the status row says why.
- Pan and zoom as in RIP: mouse wheel, Space+drag or middle drag, **F** to fit.

### History

**History**, at the top left of the canvas, lists the pictures of the selected tile, newest on top: its original and every picture an LLM wrote for it, with who made it and when. Click a row to bring that picture back; the tile keeps its place and the newer pictures stay listed. With a sector selected, History lists the edits an LLM made to the whole sector; clicking one puts all its tiles back to that edit, and the bottom row brings back what they had before. Select a row and press **Ctrl+C**, then **Ctrl+V** to paste it into the active sector as a new tile (a sector row pastes all its tiles, keeping their layout). A paste is undone with **Ctrl+Z**; going to a History row is not an undo step. Moving, scaling and rotating are not part of History.

### Guides

Every sector shows a graph-paper grid: major lines every 1/8 of the sector, minor ones every 1/64.

**LOAD...** in the **Guides** panel puts a UV-layout PNG (for example from Blender's *Export UV Layout*) on a sector, under the tiles. A file without a number goes onto the selected sector. A set named `name.1001.png`, `name.1002.png`, `name.1011.png`... goes to the sectors by number: 1001 is the bottom-left sector of the whole grid, and missing sectors are created. A guide is stretched to the sector; a non-square one is stretched too, with a warning. **REMOVE** clears the selected sector's guide. **Show** and **Opacity** apply to all guides of the workspace. Hold **G** to see every guide above the tiles in magenta. Guides and the grid are never exported.

### Export

Select the sectors to export (Shift+click for several; with none selected, the last selected sector is used), pick a scope in the **Export** panel, set the file name and press **EXPORT** to choose a folder.

| Scope | Result |
|---|---|
| **SECTOR** | One PNG per sector: `name.png` for one sector, numbered like a set for several |
| **GROUP FILE** | One sheet covering the selected sectors; unselected cells stay transparent. At most 16384 px per side |
| **GROUP SET** | One PNG per sector, `name.1001.png`, `name.1002.png`, `name.1011.png`... |

Numbers follow UDIM: 1001 is the bottom-left sector of the selection, +1 per column to the right, +10 per row up, at most 10 columns. **Fill** sets the empty space: **TRANSPARENT**, or **EXTEND**, which spreads the colour of the nearest tile up to **Edge, px** (16 by default) so mipmaps get no dark seams. Each sector is filled on its own, so colour never crosses a sector border and all three scopes give the same pixels. Existing files are never overwritten: the whole export gets `_2`, `_3` and so on, so a set keeps one name (`name_2.1001.png`).

## Connect an LLM (MCP)

An LLM client such as Claude Code, Codex or Gemini CLI can restyle atlas tiles with its own means: its image generation, skills such as ComfyUI, or other programs. ripper gives it the pictures and takes the new ones back; it never edits pictures itself and uses no network. The connection is the MCP server `ripper-mcp.exe`, installed next to `ripper.exe` in `%LOCALAPPDATA%\ripper`. It works on the current workspace, with or without a ripper window open.

**Claude Code** (PowerShell):

```powershell
claude mcp add --scope user ripper -- "$env:LOCALAPPDATA\ripper\ripper-mcp.exe"
```

**Codex**, in `%USERPROFILE%\.codex\config.toml` (put in your user name):

```toml
[mcp_servers.ripper]
command = 'C:\Users\<you>\AppData\Local\ripper\ripper-mcp.exe'
```

**Gemini CLI**, in `%USERPROFILE%\.gemini\settings.json`:

```json
{ "mcpServers": { "ripper": { "command": "C:\\Users\\<you>\\AppData\\Local\\ripper\\ripper-mcp.exe" } } }
```

Then ask in plain words, for example "pixelize the selected tiles to 64 px" or "make sector 1002 look like watercolour".

- **What it works on.** Selected tiles in ATLAS; else the selected sectors; with nothing selected, the whole atlas, and the LLM asks you first. A tile or sector you name in the request wins. The choice is fixed when the work starts.
- **New pictures.** The LLM shows you its plan and thumbnails, and writes after your yes. A new picture keeps the tile's place and size, and the shape cut out in RIP stays transparent unless the LLM is told otherwise. Pixel-art pictures are drawn with hard pixels.
- **Sectors.** A sector is edited tile by tile, or as one picture that ripper cuts back into its tiles. In the second way, tiles turned by a free angle or lying across the sector edge are left as they are and named.
- **Layout.** The LLM cannot move, scale, rotate or delete tiles, change sectors or export. It can only propose such changes: ATLAS shows a bar with its words, the moved tiles half transparent at their new place with a dashed outline, deleted tiles crossed out and new sectors dashed. **APPLY** makes the changes (one **Ctrl+Z** undoes them); **DISCARD** drops them. A newer proposal replaces one you have not answered.
- **Tools** the LLM sees: `atlas_info`, `get_scope`, `get_tile`, `render_sector`, `write_tile`, `write_sector`, `go_to_state`, `propose`.

## Troubleshooting

- **The hotkey does nothing.** Another program may own the combination. Choose a different one in **Hotkey settings**.
- **ShareX continues with the original capture after Enter.** Check that **Output file name extension** is `tex.png` and the action is ticked. A red status in ripper means a write failed; read it before closing.
- **Nothing happens after the capture.** Both **Save image to file** and **Perform actions** must be ticked; actions only run on a saved file.
- **The window does not appear.** Install or repair the WebView2 Runtime.
- **The clipboard image has no transparency in some app.** ripper puts both PNG and bitmap formats on the clipboard. Apps that only read the bitmap drop the alpha.

## Uninstall

1. ShareX **Hotkey settings**: select the ripper row and press **Remove**.
2. Windows **Settings > Apps > Installed apps**: find **ripper** and choose **Uninstall**.
3. Your settings stay in `%APPDATA%\ripper`, so a later reinstall keeps your folders. Delete that folder to remove them too.

## Building from source

1. Install the build tools, once:
   - [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the **Desktop development with C++** workload. A full Visual Studio with that workload also works.
   - Rust, via [rustup](https://rustup.rs/) or `winget install --id Rustlang.Rustup --exact`.
   - The Tauri CLI: `cargo install tauri-cli --locked`.

2. Build a portable exe without an installer:

   ```bash
   git clone https://github.com/ozymandi/Grabber.git
   cd Grabber/src-tauri
   cargo tauri build --no-bundle
   ```

   The result is `src-tauri	arget\release\ripper.exe`. A self-built copy does not update itself.

### Publishing a release (maintainers)

`scripts/release.ps1` builds the signed installer, writes `latest.json` and publishes both to GitHub Releases. It needs the updater signing key in the user environment variables `TAURI_SIGNING_PRIVATE_KEY` (path to the key file) and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Raise `version` in `src-tauri/Cargo.toml` first; it is the only place the version is set.

```powershell
.\scripts\release.ps1 -DryRun   # build and write latest.json, publish nothing
.\scripts\release.ps1           # build, confirm, publish v<version>
```

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
