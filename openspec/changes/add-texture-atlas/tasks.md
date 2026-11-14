# Tasks

## 1. Workspace model and storage (Rust, with tests)

- [x] 1.1 Create the Tauri-free module `src-tauri/src/atlas/`. It holds the manifest types (D2, `format: 1`), serde, and the helpers for sector↔canvas coordinates and UDIM numbers. Verify with unit tests:
  - the corner pixels of sectors (0,0), (1,0), (0,1) and (-1,0) land where D2 says;
  - `udim(u - u_min, v - v_min)` gives 1001, 1002 and 1011.
- [x] 1.2 Implement the working-folder store (D1):
  - create and open a workspace;
  - write `items/<id>.png` before the manifest;
  - write `manifest.json` atomically through tmp and rename;
  - keep `index.json` with `saved_revision`, and compute "unsaved".

  Verify with tests: a fresh workspace has one sector, size 2048 and padding 8; after one edit the workspace is unsaved and `revision` has grown.
- [x] 1.3 Implement the lock and mutation pipeline (D3):
  - `manifest.lock` holds the pid and a timestamp;
  - writers retry for 2 s;
  - a stale lock is broken;
  - every operation is lock → re-read → apply → write.

  Verify with a test where two threads each add 50 tiles to one workspace: the result has 100 tiles and no lost item. Verify also that a held lock makes an add fail after about 2 s with an error, not a hang.
- [x] 1.4 Implement the `.ripper` archive (D1):
  - SAVE and SAVE AS through a tmp file and rename, purging unreferenced items;
  - OPEN, with the recovery rule: a working folder with unsaved changes wins over the file.

  Verify with a test that six tiles survive save → new workspace → open with the same rects and rotations (spec "Save and reopen"), and that the purged item is gone from the archive and from the working folder.
- [x] 1.5 Extend `settings.rs` with `atlas_on` (default false) and `current_workspace` (default empty). Verify with tests that a 1.0.0 `config.toml` without these fields loads with the defaults, and that both fields round-trip.

## 2. Placement and tile operations (Rust, with tests)

- [x] 2.1 Implement `place()` with the candidate search (D4): active sector first, then UDIM order; fitting down, rounding the size down; the tray beside the grid. Verify with tests for the spec scenarios:
  - "Tile fits in the active sector": no overlap, padding kept, existing tile unmoved;
  - "Active sector full" goes to 1002;
  - "Large capture into a small sector" gives 1008 x 604 with the original 1500 x 900 kept;
  - "All sectors full" goes to the tray and reports it;
  - automatic placement never crosses a sector border.
- [x] 2.2 Implement the tile operations:
  - move;
  - scale from a corner, about the opposite corner, minimum 1 x 1;
  - rotate ±90° about the centre;
  - delete, returning the count.

  Implement the sector operations: add on a free side, remove only when no tile overlaps, size change only when empty, padding change without moving tiles, set the active sector.

  Verify with tests for the spec scenarios "Scale with a corner handle", "Rotate a tile", "Delete two tiles", "Add a sector to the right", "Remove a sector", "Sector with tiles", "Size locked after the first tile" and "Padding applies to new tiles only". Verify also that four rotations restore the exact placement.

## 3. Export rendering (Rust, with tests)

- [x] 3.1 Render a tile at its placed size from the original with `warp::warp` (D5). Add an exact index-mapping fast path for scale 1 at any quarter turn, and test it against the warp. Verify with tests:
  - rotating four times reproduces the original bytes;
  - scaling down and back to full size reproduces the original bytes (spec "Shrink and enlarge again");
  - a fine stripe pattern shrunk to a quarter stays within the same error bound as the RIP quality test (spec "Shrunk fine pattern").
- [x] 3.2 Compose the outputs with clipping and premultiplied source-over in manifest order, onto a transparent start:
  - SECTOR, one PNG per sector;
  - GROUP FILE: bounding rectangle, unselected cells transparent, limited to 16384 px;
  - GROUP SET: UDIM numbering from the bottom-left, refused beyond 10 columns.

  Naming adds `_2`, `_3`… to the base name, the same suffix for the whole set. Verify with tests for the spec scenarios:
  - "Export a single 2048 sector", "Two by two sheet", "L-shaped selection", "Sheet too large";
  - "Set numbered from the bottom-left", "Too wide for UDIM";
  - "Empty sector", "Tile on a border" (the halves join without a gap or overlap), "Name collision in a set";
  - tray tiles never appear.

## 4. Tauri integration

- [x] 4.1 Add Tauri commands for:
  - the workspace: get, new, open, save, save as, set size, set padding;
  - sectors: add, remove, select;
  - tiles: move, scale, rotate, delete, select-free queries if needed;
  - export.

  Each command returns the new manifest. Add `ripper://atlas/<ws>/<tile>?max=2048`, which serves cached thumbnails from `thumbs/` (D5). Verify that `cargo build` and `cargo test` pass, and that a debug run serves a thumbnail at most 2048 px on its long side.
- [x] 4.2 Add the Atlas output to `export` (D7). Rust reads `atlas_on` and `current_workspace` and adds each rendered quad after the clipboard, ShareX output and folders. `ExportResult.atlas` reports added, tray and error. `exit_after` also requires that the atlas succeeded. Verify with a unit test of the exit rule (ShareX mode plus atlas error means stay open) and that `cargo test` passes.

## 5. UI (ATLAS mode)

- [x] 5.1 The glyph for the sector "+" buttons. O'Tool has none; the designer chose Lucide (`plus`, lucide-static 1.72.0, as published), with no Unicode or PNG stand-in. Add it to `icons.js` with the same named-deviation note as the update icon. Verify by measurement in `?demo` that its size and hover match the O'Tool icon buttons.
- [x] 5.2 Add the RIP | ATLAS `seg-row` at the top of the side panel and `#atlasStage` (D6). Switching keeps the RIP image and quads, and a ShareX launch always starts in RIP. Verify through CDP in the real app the spec scenarios "Switch to ATLAS" (the RIP state is identical after switching back) and "ShareX opens RIP".
- [x] 5.3 Build the atlas canvas:
  - WebGL2 tiles from the thumbnails, sector fills, overlay with borders, selection and handles;
  - pan and zoom as in RIP, F to fit;
  - polling `revision` every 500 ms while ATLAS is visible.

  Verify in the real app the spec scenario "Capture arrives while the atlas is open": the tile appears within 2 s and a tile moved just before keeps its position. Verify also that dragging on a 4K-thumbnail workspace keeps 60 fps.
- [x] 5.4 Add the interactions:
  - tiles: click and Shift+click selection, drag, corner-handle scaling with the live size in the status, R and Shift+R, Delete, Esc that clears and never quits in ATLAS;
  - sectors: selection and Shift selection, "+" buttons on free sides, REMOVE with a reason when blocked.

  Verify each through CDP against the atlas-tiles and atlas-workspace scenarios. Verify also that keys use `e.code` and work on the Ukrainian layout.
- [x] 5.5 Add the ATLAS panels in O'Tool components:
  - **Workspace:** name with the unsaved mark; NEW, OPEN, SAVE, SAVE AS; size select, disabled with a reason when there are tiles; padding;
  - **Sector:** UDIM number, REMOVE;
  - **Export:** count, scope seg SECTOR / GROUP FILE / GROUP SET, base name, EXPORT through the folder picker;
  - the status texts.

  Add `.ripper` filters to the open and save dialogs. Show the panel layout and the status wording to the designer for approval. Verify the spec scenarios "Save and reopen", "Window closed before saving", "ShareX tile then save" and "Select three sectors" in the real app.
- [x] 5.6 Add the Atlas slot to the RIP Output panel (checkbox, workspace name). Route keys by mode: Ctrl+S saves all in RIP and saves the document in ATLAS. Add the ATLAS card to help. Verify through CDP the texture-export scenarios "Send adds a tile to the atlas", "Save all into the atlas", "Atlas output off" and "Atlas toggle survives restart".

## 6. End to end and release

- [x] 6.1 Run end to end with real ShareX (Ctrl+Shift+F11) and the Atlas output on:
  - Enter adds a tile and the app exits; the tile shows in the atlas;
  - with the working folder made read-only, the output, clipboard and folders are written, a warning names the atlas and the app stays open (spec "Workspace cannot be written");
  - export of each scope is checked with pixel probes on the written PNGs.

  Restore the folder's permissions afterwards.
- [x] 6.2 Measure:
  - a ShareX Enter with the Atlas output on, from a 4K source: the added time stays under 300 ms;
  - ShareX-mode startup: stays at 0.57–0.59 s, as in 1.0.0;
  - save of a workspace with twenty 4K originals.

  Record the numbers in `task.md`.
- [x] 6.3 Add the ATLAS section to README: modes, workspace and files, sectors, tiles, export scopes, UDIM numbering. Verify that every label in the README matches the app.
- [ ] 6.4 Set `version = "2.0.0"`, commit, push, run `scripts/release.ps1 -DryRun`, then publish once the designer says yes in chat. Verify on this machine that the installed 1.0.0 shows the update button, installs 2.0.0 on click and relaunches as 2.0.0. This is spec app-distribution "Install update" on real GitHub releases. Verify also that ShareX keeps working afterwards.
- [ ] 6.5 Update `task.md` (status, timings, next step 2.1.0) and the project memory. Verify that both name the 2.0.0 release and the 2.1.0 scope.

## Workflow follow-up

- Archive the change after the designer reviews the release.
- 2.1.0, a separate change: UV-layout guides, free rotation, edge dilation on export.
