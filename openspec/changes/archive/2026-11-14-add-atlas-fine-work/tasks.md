# Tasks

## 1. Tile frame and format 2 (Rust, with tests)

- [x] 1.1 Replace the tile rect with the D1 frame in `model.rs`: `c2`, `w`, `h`, `rot`, `tilt`, `pivot`. Add a derived bounding box and switch `place.rs`, `tiles_on`, sector removal and the tray to it. Verify with the existing 2.0.0 tests, all passing unchanged in meaning, plus a test that a tilted tile's bounding box covers all four rotated corners.
- [x] 1.2 Add the format 1 → 2 upgrade on read and `Error::NewerFormat` for `format > 2`, in both `Workspace::read` and `Store::open`. Verify with tests:
  - a format-1 manifest with tiles at `rot` 0–3 gives the same bounding boxes and the same exported pixels as 2.0.0 (spec "Open a 2.0.0 file");
  - a `format: 3` file is refused and the current workspace is unchanged.
- [x] 1.3 Implement R about the pivot in doubled integers (D1). Verify with tests:
  - spec "Rotate a tile" (400 x 200 → 200 x 400, centred);
  - "Four turns about a moved pivot" returns the exact frame, for pivots at a corner, off the tile and at an odd-size centre;
  - "Rotate about a corner" keeps the corner in place;
  - "Quarter turn on a free angle": 17° becomes 107°.
- [x] 1.4 Implement `turn {id, angle}` (free angle about the pivot, split into `rot` and `tilt`, near-zero tilt stored as 0) and `set_pivot {id, pivot}`, which rounds to whole pixels; `None` resets to the centre. Verify with tests: turning by 30° and back gives a frame within 0.5 px; a 90° total gives `tilt == 0`; the pivot moves with the tile after `move`.
- [x] 1.5 Implement scaling along the tile's axes with `anchor: corner | pivot` (D1). The pivot scales proportionally. Verify with tests for spec "Scale with a corner handle", "Scale a rotated tile" (opposite corner fixed within 0.5 px, angle unchanged) and "Scale about the pivot" (centre unchanged), plus a minimum of 1 x 1.

## 2. Guides and workspace settings (Rust, with tests)

- [x] 2.1 Add `guides`, `guide_show`, `guide_opacity`, `fill` and `edge` to the manifest with the D4 defaults. Add the edits `set_guide_view`, `set_fill` and `set_guide`/`remove_guide`. `remove_sector` also drops the sector's guide. Verify with tests:
  - defaults on a new and on an upgraded workspace;
  - the edge range is 1..sector size;
  - spec "Remove a sector with a guide".
- [x] 2.2 Make SAVE pack and the cleanup keep items referenced by tiles or guides, and make OPEN unpack guide items. Verify with a test for spec "Guides travel with the file": save → NEW → open restores guides, Show, Opacity, Fill and Edge.
- [x] 2.3 Implement `load_guides(paths, target)` (D4):
  - classify file names, refusing several un-numbered files;
  - write items before the manifest;
  - map 1001 to the grid's bottom-left under the lock;
  - create missing sectors;
  - replace existing guides;
  - report the loaded, created and not-square counts.

  Verify with tests for spec "Set into a one-sector workspace", "Numbering ignores the selection", "Second row", "Guide on the selected sector", "Replace a guide" and "Non-square guide".

## 3. Rendering: tilted tiles and edge fill (Rust, with tests)

- [x] 3.1 Add `min_sub` to `warp::warp`: RIP and exact tiles use 1, so their output is byte-identical to before. Add the tilted path of `render_tile` (D2). Verify with tests:
  - all existing warp and render tests are unchanged;
  - spec "Freely rotated tile": a 30° tile has partial-alpha edge pixels and a transparent outside;
  - 0/90/180/270° stay pixel-exact.
- [x] 3.2 Implement the per-cell edge fill (D3): seeds with alpha ≥ 128, an exact EDT with argmin, the underlay plus over, limited to `edge`. Run it per cell for SECTOR, GROUP SET and GROUP FILE when `fill == extend`. Verify with tests for spec:
  - "Extend by 16 pixels";
  - "Two tiles meet halfway";
  - "Soft edge becomes solid": no pixel within the edge has alpha < 255;
  - "Tile near a border": no colour crosses into 1002;
  - "Same pixels in every scope";
  - "Empty sector" stays transparent with EXTEND.
- [x] 3.3 Measure release export times on this machine: a 4096 sector with 20 tiles, 5 of them tilted, with TRANSPARENT and with EXTEND; GROUP FILE 8192² with EXTEND. Record them in `task.md`. Verify: a 4096 sector with EXTEND finishes in under 1 s.

## 4. Commands and undo plumbing (Rust/Tauri, with tests)

- [x] 4.1 Make every forward edit return its inverse and a label, computed under the lock before applying (D6). Inverses: `set_frame`, `restore_tile` (same id and z-index), `add_sectors`/`remove_sectors` (with guides) and `set_guide`. Settings edits return no inverse. Verify with model tests: for each operation, applying forward then inverse restores the manifest except `revision`.
- [x] 4.2 Add `atlas_apply_steps(edits)`: best effort in one lock and one write. It returns the doc, the skipped reasons and the inverse of what it applied. Verify with tests for spec:
  - "Sector received a tile": the sector stays, the reason is reported, the step is consumed;
  - "Tile deleted in another window";
  - "Capture arrives between edits": the ShareX tile survives undo.
- [x] 4.3 Add the `held` item set to AppState (D6) and make `save_as` skip held items in its cleanup without packing them; NEW and OPEN clear it. Verify with a test: delete → save → undo restores a tile whose original is still on disk, and a later save packs it.
- [x] 4.4 Wire the Tauri commands `atlas_load_guides`, `atlas_apply_steps` and the new `atlas_edit` variants. Serve guide items through `ripper://atlas/<ws>/<item>?max=4096`. Verify with `cargo test` and `cargo clippy` clean, and that a guide URL returns a PNG in the running app.

## 5. ATLAS canvas (UI)

- [x] 5.1 Draw the graph-paper grid and the guide layer in the D5 order, with Show and Opacity applied, and the hold-G magenta preview with lazy tinted copies cleared on blur. Verify in the app via CDP for spec:
  - "Grid on a 2048 sector" (256/32 px at 100 %) and "Zoomed out" (minor lines hidden);
  - "Tiles cover the guide";
  - "Check the UV islands": the preview turns on with G and off with G or blur.
- [x] 5.2 Draw tiles with free rotation, frames and handles on rotated corners, and hit-tests in tile-local space. Verify for spec "Click beside a rotated tile" and that dragging 20 tiles (5 tilted) with guides stays at 60 fps.
- [x] 5.3 Add the rotate zone with the Lucide `rotate-cw` cursor (1.72.0, as published; literal `--ink`/`--bg` colours noted in `icons.js`), Shift 15° snapping and the angle in the status. Verify for spec "Turn a tile by dragging", "Snap with Shift" and "Handle and zone stay apart".
- [x] 5.4 Add the pivot: Lucide `crosshair` geometry, Alt+drag with snapping to nine points, Alt+double-click reset, Alt suppressed from focusing the window chrome. Add Alt+handle scaling about the pivot. Verify for spec "Rotate about a corner", "Reset the pivot", "Pivot is kept" and "Scale about the pivot" in the app.

## 6. ATLAS panels, undo stacks, keys and docs (UI)

- [x] 6.1 Add the Guides panel between Sector and Export (D5): head with the sector name, LOAD... with a multi-file PNG picker, REMOVE with its disabled state, Show and an Opacity `slider-capsule`. Verify for spec "Panel follows the selection", "Opacity is kept" and the LOAD status texts.
- [x] 6.2 Add the Fill `seg-row` (TRANSPARENT / EXTEND) and `Edge, px` to the Export panel, with Edge disabled under TRANSPARENT, stored through `set_fill`. Verify for spec "Setting is kept".
- [x] 6.3 Implement the history in `atlas.js` (D6):
  - two stacks capped at 100;
  - Ctrl+Z, Ctrl+Shift+Z and Ctrl+Y by `e.code`;
  - status labels and skipped reasons;
  - "nothing to undo" and "nothing to redo";
  - redo cleared on a new step;
  - both stacks cleared on NEW and OPEN.

  Verify via CDP for spec "Undo a move", "Undo a delete", "Settings are not steps", "Undo a set load", "Undo after saving", "Open starts fresh", "Back to the saved state" and "Cyrillic layout".
- [x] 6.4 Update the help overlay's ATLAS card (rotate zone, Shift, pivot and Alt, G preview, Guides, Fill, Ctrl+Z, Ctrl+Y) and the README ATLAS section, including that 2.1 files need 2.1. Verify by reading both against the specs.
- [x] 6.5 Designer review of the new panels, the grid colours, the pivot glyph, the rotate cursor and the magenta preview. Record the approved choices and every named deviation (magenta, literal cursor colours) in `task.md`. Verify with the designer's explicit approval.

## 7. Integration and release

- [x] 7.1 Run the end-to-end check on a release build:
  - open a 2.0.0 `.ripper` file;
  - load a `1001–1004` guide set;
  - turn tiles freely about moved pivots;
  - export all three scopes with EXTEND;
  - undo back to the opened state;
  - a ShareX capture arrives meanwhile and survives.

  Verify the exported pixels against the unit-test expectations and that the 2.0.0 file's tiles are untouched.
- [x] 7.2 Bump `src-tauri/Cargo.toml` to 2.1.0, run `scripts/release.ps1 -DryRun`, and publish only after the designer's explicit yes. Then update this machine through the button and check a ShareX capture. Verify with the release URL and the installed version.
- [x] 7.3 Update `task.md` (status, timings, next step) and the project memory. Verify that both name the 2.1.0 release.

## Workflow follow-up

- Archive the change with `/opsx:archive add-atlas-fine-work` after the release; this creates the main specs `atlas-guides` and `atlas-undo` and updates `atlas-tiles`, `atlas-export` and `atlas-workspace`.
