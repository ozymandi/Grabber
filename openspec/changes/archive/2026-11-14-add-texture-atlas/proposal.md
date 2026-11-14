# Proposal

## Why

ripper turns each capture into a separate PNG. Texturing a model, though, needs those pieces gathered onto sheets that line up with the model's UV tiles. Today that assembly happens by hand in another program, one capture at a time. Version 2.0.0 adds an atlas editor inside ripper. Captures, including ShareX captures, land on a UDIM-style grid of power-of-two sectors. There they are arranged and exported as single sheets or as numbered tile sets.

This change is the 2.0.0 core. Fine work comes as 2.1.0: UV-layout guides, free rotation and edge dilation. The split keeps the first release to lossless pixel moves plus scaling from the original, which lowers the risk of critical bugs.

## What Changes

- **Two modes in one window:** RIP, which is today's editor, and ATLAS, the new atlas editor. A RIP | ATLAS segmented switch sits at the bottom-left of the canvas area. A ShareX launch always opens in RIP.
- **Workspace document.**
  - A workspace is an infinite canvas holding sectors and tiles. It is saved as a single `.ripper` file.
  - The current workspace is the one opened last, and new tiles go there. NEW starts an untitled workspace, OPEN loads a `.ripper`, SAVE and SAVE AS write the file.
  - Every edit is stored at once in a working folder under `%APPDATA%\ripper`, so nothing is lost if the window closes. The `.ripper` file itself is updated on save.
- **Sectors.**
  - Each workspace has one sector size: 512, 1024, 2048, 4096 or 8192, square.
  - A new workspace starts with one sector. "+" buttons on a selected sector's free sides add a neighbour flush against it. Together they form a UDIM grid.
  - An empty sector can be removed.
- **Tiles.**
  - **Arrival.** A tile auto-packs into the free space of the active sector, keeping the workspace padding. Existing tiles never move. A tile larger than the sector is scaled down to fit. If it does not fit anywhere, it goes to the tray: free canvas outside all sectors, which is never exported.
  - **Editing.** Tiles can be selected, dragged, scaled with corner handles from the full-size original, rotated by 90° and deleted. A tile may lie across sector borders.
- **Export.** Three scopes:
  - one sector;
  - selected sectors as one file;
  - selected sectors as a UDIM set `name.10xx.png`, numbered from the selection's bottom-left sector.

  Empty space is transparent. Existing files are never overwritten.
- **RIP mode gets a third output, "Atlas",** next to Local and Google Drive. It has its own checkbox and shows the current workspace name.
  - Enter, including from ShareX, adds the active quad to the atlas.
  - Ctrl+S in RIP adds every quad.
  - Ctrl+S in ATLAS saves the document.
- **Version 2.0.0.** It is published with `scripts/release.ps1` and reaches installed 1.0.0 copies through the updater. This is the first real end-to-end use of the update path.

Out of scope for 2.0.0:
- **Moved to 2.1.0:** UV-layout guide layers, free rotation, edge dilation on export.
- **Not planned yet:** undo and redo, snapping, non-square sectors, changing the sector size of a workspace that already has tiles, per-workspace texel density, seamless tiling, trim sheets, an engine-specific JSON manifest.

## Capabilities

### New Capabilities
- `atlas-workspace`: the atlas document and its canvas. Covers:
  - the RIP | ATLAS modes;
  - new, open, save and save-as, the current workspace, the working folder and recovery after a closed window;
  - sector size, adding and removing sectors and the UDIM grid;
  - the active sector, the workspace padding;
  - staying consistent when a ShareX instance adds tiles while the atlas is open.
- `atlas-tiles`: tiles inside a workspace. Covers:
  - how a new tile is placed: auto-packing into the active sector, fitting down, the tray;
  - keeping the original;
  - selecting, moving, scaling and 90° rotation;
  - deleting;
  - tiles across sector borders.
- `atlas-export`: turning sectors into image files. Covers:
  - the three scopes: one sector, a group as one file, a group as a UDIM set;
  - UDIM numbering and its limits;
  - how tiles are rendered into the image (scale from the original, cropping at sector edges, transparent fill);
  - file naming without overwrite.

### Modified Capabilities
- `texture-export`: the Atlas output. Send and Save All also add tiles to the current workspace when the Atlas output is on. The Atlas toggle persists with the other settings. An atlas write failure is reported like a folder failure and does not block the other outputs.

## Impact

- **Rust (Tauri-free core in `src-tauri/src/`):**
  - new `atlas/` module: model and manifest, sector grid, packer, document storage and packing into `.ripper`, sector rendering and export;
  - `export.rs` and `main.rs` are extended for the Atlas output;
  - `settings.rs` gains the Atlas toggle and the current-workspace pointer.
- **Rust (Tauri side):** new commands for the workspace, sectors, tiles and export. The existing `ripper://` protocol also serves tile images.
- **UI:**
  - `index.html` gets the mode switch and the ATLAS panels: Workspace, Sector, Export;
  - new `ui/js/atlas/` holds the canvas view, tile interaction and panels;
  - `panel-output.js` gets the Atlas slot;
  - `main.js` routes keys by mode;
  - help gets an ATLAS card.
- **Dependencies:** `zip`, already in the tree through the updater, becomes a direct dependency for `.ripper` files. No new frontend libraries.
- **Files on disk:** working folders under `%APPDATA%\ripper\workspaces\`, and user `.ripper` files wherever they are saved.
- **Docs:** README (ATLAS section) and `task.md`.
- **Release:** `version = "2.0.0"`, published with `scripts/release.ps1`.
