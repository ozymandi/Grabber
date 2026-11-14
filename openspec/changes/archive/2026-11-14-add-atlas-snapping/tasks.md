# Tasks

## 1. Snap computation (pure JS, with tests)

- [x] 1.1 Create `ui/js/atlas/snap.js` (design D5) with `gridStep(size, k, dpr)`. Switch `drawGrid` in `canvas.js` to it so grid drawing and snapping share one visibility rule. Verify that the grid looks unchanged in the app at 100 % and zoomed out (spec atlas-guides "Grid on a 2048 sector", "Zoomed out").
- [x] 1.2 Implement `targets(manifest, selectedIds)` (D2):
  - sector borders, flush and at padding inside;
  - unselected tile boxes from `F.rect`, flush and at padding outside;
  - spans for the snap lines;
  - padding-0 duplicates removed.

  Verify with `ui/js/atlas/snap.test.mjs`: targets for one sector and two tiles with padding 8 and 0; selected tiles are excluded.
- [x] 1.3 Implement `snapAxis(lo, hi, targets, grid, k, dpr)` (D1): the 8 × dpr screen threshold, edges winning over the grid, nearest otherwise, integer correction. Verify in `snap.test.mjs`:
  - spec "Flush to a sector border" and "At the padding from a sector border";
  - "Beside another tile at the padding";
  - "Tile edge wins over the grid" and "Hidden minor lines do not attract";
  - "Threshold follows the zoom";
  - "Turned tile": a 45° tile's box snaps flush;
  - "Several tiles move as one box".

  Run with `node ui/js/atlas/snap.test.mjs`, exit 0.

## 2. Canvas wiring (UI)

- [x] 2.1 In `canvas.js`, take the selection box and targets at the start of a move drag. In `onMove`, apply `snapAxis` per axis to the raw `dx, dy` unless `e.ctrlKey` is held, and keep the snap results in the drag state. `onUp` still sends one `move { ids, dx, dy }`. Verify in the app via CDP for spec "Flush to a sector border", "Axes snap on their own", "Snap lines end with the drag" and "One undo step".
- [x] 2.2 Track Ctrl by `e.code` (`ControlLeft`/`ControlRight`) on keydown/keyup during a move drag (D3), and recompute from the last pointer position. Resync from `ctrlKey` on the next pointer event. Verify in the app for spec "Ctrl turns snapping off": the tile jumps with no pointer movement; Ctrl alone triggers no Ctrl+letter action.
- [x] 2.3 Draw the snap lines (D4): `--danger` read with the other canvas colours, 1 device pixel, spanning the target and the moving box, only during a move drag. Verify via a screenshot of a snapped drag beside a tile and against a sector border, and for spec "Scaling does not snap" that no line appears while scaling.
- [x] 2.4 Add "Ctrl+drag — Move without snapping" to the help overlay's key list and one sentence to its ATLAS card in `ui/index.html`. Add the same to the README Atlas section. Verify by reading both against the spec.
- [x] 2.5 Designer review of the snapping feel, the threshold and the `--danger` snap line. Record the outcome and the named use of `--danger` in `task.md`. Verify with the designer's explicit approval.

## 3. Integration and release

- [x] 3.1 Run the end-to-end check on a release build in a separate test workspace:
  - 2 × 2 sectors with 10 tiles, 2 of them turned;
  - pack them by snapping at padding 8 and at padding 0;
  - export GROUP SET with EXTEND;
  - undo several drags;
  - select a packed row by frame (change `add-atlas-marquee`, released together in 2.2.0) and move it with snapping.

  Run it after `add-atlas-marquee` group 2 is done. Verify that the exported gaps equal the padding exactly, at 60 fps while dragging, and that the designer's own workspace is untouched.
- [x] 3.2 Bump `src-tauri/Cargo.toml` to 2.2.0, run `scripts/release.ps1 -DryRun`, and publish only after the designer's explicit yes. Then update this machine through the button. Verify with the release URL and the installed version.
- [x] 3.3 Update `task.md` (status, next step) and the project memory. Verify that both name the 2.2.0 release.

## Workflow follow-up

- Archive the change with `/opsx:archive add-atlas-snapping` after the release. This adds the requirement "Snap while moving" to the main spec `atlas-tiles`.
