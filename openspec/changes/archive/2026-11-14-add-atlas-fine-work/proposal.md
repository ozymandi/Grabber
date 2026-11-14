# Proposal

## Why

ripper 2.0.0 lets the user gather captures into a UDIM atlas. Fine texturing work, though, still happens elsewhere:
- the user cannot see the model's UV islands while placing tiles;
- tiles turn only in quarter steps;
- exports have hard transparent edges that bleed dark seams into mip levels;
- one wrong Delete or drag cannot be taken back.

2.0.0 deliberately left these out to stay lossless and low-risk. 2.1.0 adds them now that the core is released and stable.

## What Changes

- **UV-layout guides.** A new Guides panel in ATLAS, between Sector and Export.
  - **Loading.** LOAD... takes one or many PNGs. A PNG without a number goes onto the selected sector, or onto the active sector if none is selected. A `*.10xx.png` set is spread by number: 1001 is the bottom-left cell of the whole workspace grid, and missing sectors are created. A new guide replaces an existing one. A set wider than 10 sectors is refused. REMOVE clears the selected sector's guide.
  - **Display.** A guide lies under the tiles, stretched to the sector square. Show and Opacity are per workspace, on and 50 % by default. Holding G shows every guide above the tiles as a magenta mask until G is released.
  - Guides are stored in the `.ripper` file and never exported.
- **Graph-paper grid.** Every sector always shows a grid under everything: major lines every 1/8 of the sector, minor lines every 1/64. Minor lines fade out when they would be denser than ~6 screen pixels. The grid is never exported.
- **Free rotation.**
  - **Gesture.** Drag in a zone just outside a selected tile's corner to rotate it; Shift snaps to 15°. The status shows the angle. R and Shift+R still add ±90° to any angle, and 0, 90, 180 and 270° stay lossless.
  - **Pivot point.** Each tile has its own pivot, at the centre by default and stored in the workspace. Alt+drag moves it, and it snaps to corners, edge midpoints and the centre. Alt+double-click resets it. R and free rotation turn about the pivot.
  - **Scaling a rotated tile.** The frame and handles rotate with the tile. A handle scales along the tile's own axes about the opposite corner; Alt+handle scales about the pivot. Clicks hit the rotated tile itself, not its bounding box.
- **Edge fill on export.** The Export panel gains Fill: TRANSPARENT | EXTEND and Edge, px (16 by default).
  - EXTEND spreads each tile's edge colour up to N px into empty space; each empty pixel takes the colour of the nearest tile.
  - It is computed per sector, so all three export scopes give identical pixels for a sector.
  - Fill and Edge are stored in the workspace.
- **Undo in ATLAS.**
  - **Keys.** Ctrl+Z undoes; Ctrl+Shift+Z or Ctrl+Y redoes.
  - **What is undoable.** Content changes made in this window: move, scale, rotation, pivot, Delete, adding and removing sectors, loading and removing guides. One gesture is one step. Tiles that arrive from ShareX or from RIP are not undone.
  - **Lifetime.** History lives in the window's memory, up to 100 steps. It survives SAVE; NEW and OPEN start a fresh one.
  - **Conflicts.** A part of a step that can no longer be undone is skipped with an explanation in the status; history never gets stuck.
- **File format 2.**
  - The manifest stores the free angle, pivots, guides, guide view settings and edge fill.
  - Workspaces and `.ripper` files from 2.0.0 open unchanged and are upgraded when written.
  - **BREAKING (forward only):** files saved by 2.1.0 are not meant for 2.0.0. The updater keeps installs current, and 2.1.0 refuses files from a newer format with an explanation.
- **Version 2.1.0.** It is published with `scripts/release.ps1` after the designer's yes and reaches installed copies through the updater.

Out of scope:
- undo in RIP; history kept on disk across launches;
- an angle field or Tile panel; tinting guides other than the magenta preview;
- snapping tiles to each other or to the grid;
- non-square sectors, seamless tiling, edge-sharing quads.

## Capabilities

### New Capabilities
- `atlas-guides`: visual aids on sectors that are never exported. Covers:
  - the Guides panel;
  - loading single guides and numbered `10xx` sets, including sector creation and the width limit;
  - removing guides;
  - guide visibility and opacity; the hold-G magenta preview;
  - how a guide is fitted to its sector;
  - the always-on graph-paper grid.
- `atlas-undo`: undo and redo of the user's own ATLAS edits. Covers what enters the history, step granularity, the 100-step in-memory lifetime and redo rules, and skipping parts that conflict with changes from elsewhere.

### Modified Capabilities
- `atlas-tiles`:
  - **free rotation and the pivot point** are new requirements;
  - **90° rotation** turns about the pivot instead of the centre;
  - **scaling** works on the rotated frame and gains Alt for pivot scaling;
  - **selection** hits the rotated outline.
- `atlas-export`:
  - **tile rendering** covers freely rotated tiles with smooth edges;
  - **empty areas** are transparent only with Fill TRANSPARENT;
  - **edge fill**: the new EXTEND fill, computed per sector.
- `atlas-workspace`:
  - **the `.ripper` document** also holds guides and the per-workspace view and export settings;
  - **2.0.0 files** open unchanged;
  - **the list of edits kept without saving** gains free rotation, pivots and guides.

## Impact

- **Rust core (`src-tauri/src/atlas/`):**
  - `model.rs`: the tile frame with half-pixel centre, quarter turns, tilt and pivot; guides and settings; new operations with inverses; format 1 to 2 upgrade;
  - `render.rs` and `warp.rs`: tilted tiles with edge antialiasing, and the edge-fill pass per sector;
  - `store.rs`: a `guides/` folder in the working folder and in the zip; items held by the undo history are kept out of the save-time cleanup;
  - `place.rs`: uses the tile's bounding box.
- **Rust (Tauri side):**
  - `atlas_cmd.rs`: new edits, a best-effort batch for undo, inverse edits in every result, a guide LOAD command;
  - the `ripper://` protocol serves guide images.
- **UI:**
  - `index.html`: the Guides panel and the Fill row in Export;
  - `ui/js/atlas/canvas.js`: grid and guide layers, rotated drawing and hit-testing, the rotate zone, pivot and Alt behaviour, hold-G preview;
  - `atlas.js`: undo and redo stacks and keys;
  - `icons.js`: Lucide `rotate-cw` as the cursor and `crosshair` for the pivot;
  - help: the ATLAS card.
- **Design-system deviation:** the magenta `#FF00FF` preview colour is not an O'Tool token (named to the designer).
- **Docs and release:** README ATLAS section, `task.md`, `version = "2.1.0"`.
