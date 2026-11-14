# Tasks

## 1. Shape geometry (Rust + JS, mirrored, with tests)

- [x] 1.1 Add `Shape` in a new `shape.rs` next to `geom.rs` (design D1): a serde enum tagged by `kind` with `Rect` as default, plus `Node { p, hin, hout, smooth }`. Add `outline(shape, w, h)` (D2): ellipse at 0.05 px chord error, N-gon per axis scaled to `[0, w] × [0, h]`, and Bézier flattening at 0.05 px. Also add `to_path(shape)` (Rect/Circle/N-gon → Path with κ handles) and `split_at(path, seg, t)` (de Casteljau). Verify with `cargo test` in `src-tauri`:
  - N = 8 touches all four sides with flat top and bottom (spec "Octagon on a STOP sign");
  - N = 3 has its apex at `(w/2, 0)` and its base on `y = h` (spec "Triangle points up");
  - the ellipse's extreme points lie on the edges;
  - Circle → Path stays within 0.0003 · r of the ellipse;
  - splitting a cubic keeps its points within 1e-9;
  - an old JSON `QuadReq` without `shape` parses as Rect.
- [x] 1.2 Mirror D1–D2 in a new `ui/js/shape.js` (`outline`, `toPath`, `split`, `unitToImage` via `squareToQuad`, and the inverse). Check it against the fixture `src-tauri/tests/fixtures/shapes.json`, generated from `shape.rs` by `tests/shape_fixture.rs`, and verify with `node ui/js/shape.test.mjs` that JS and Rust give the same vertices to 1e-9.

## 2. Mask in the export (Rust)

- [x] 2.1 Create `src-tauri/src/mask.rs` (D3): 4 × 4 supersampled, nonzero-winding coverage of a polygon over a `w × h` grid, multiplied into straight alpha. Add `Invalid::EmptyShape` for an outline area under 1 px², with the text "the shape has no area". Verify with `cargo test`:
  - a 64 × 64 Circle has opaque centre, transparent corners and partial alpha on its edge (spec "Smooth edge");
  - an 8-gon's slanted corners are transparent;
  - a zero-area path is rejected.
- [x] 2.2 Add `shape` to `QuadReq` (`#[serde(default)]`) and apply the mask in `export::render` after the warp, with an early return for Rect (D3). Verify with `cargo test`:
  - a Rect export is byte-identical to the plain warp that `render` returned before the change, on a gradient source (spec "Rectangle is unchanged");
  - a 512 × 512 Circle export matches spec "Circle on a square output".
- [x] 2.3 Add `shape` (default "rect") and `ngon` (default 8) to `settings.rs`. Verify with `cargo test` that an old `config.toml` without them loads with the defaults and that they round-trip.

## 3. History (UI)

- [x] 3.1 Create `ui/js/history.js` (D8): `record()`, `undo()`, `redo()`, `clear()`, a cap of 100, redo cleared on a new record, and dirty marking by JSON comparison. Call `clear()` from `setSource`. Verify with `ui/js/history.test.mjs` (`node`, exit 0):
  - undo and redo of create, remove (restored in place and active), a corner change and a shape change;
  - a new record clears redo;
  - the cap;
  - restored quads are dirty;
  - `setSource` clears the history.
- [x] 3.2 Record before every quad edit in `state.js` and `editor.js`: create, remove, the first move of a corner drag, shape and N, and the active quad's aspect and PoT in `panel-aspect.js`. Wire Ctrl+Z, Ctrl+Shift+Z and Ctrl+Y in the RIP branch of `main.js`, by `e.code` and ignored while typing. Verify in the app via CDP against spec "Undo a corner drag", "Undo a removal", "Undo an aspect change", "Redo", "New edit clears redo", "Undo after Send" and "New image clears the history".

## 4. Shapes in the UI

- [x] 4.1 Add `shape` to RIP quads and the defaults in `state.js`, send it in every export request, and load and save the defaults through the settings. Add the shape icon to the Quads panel header and the popup (D9), with the Lucide `square`, `circle`, `octagon` and `pen-tool` glyphs added to `icons.js` as published, and Shift+S in `main.js`. Verify in the app via CDP:
  - spec "Pick Circle in the popup" and "Cycle with Shift+S";
  - the popup closes on Esc and on a click outside, and Esc there does not close the app;
  - "New quads take the last shape" across a restart.
- [x] 4.2 Add the preview mask (D4): an offscreen 2D canvas filled with `outline`, uploaded as a second texture only when the shape or a size changes. Verify via a CDP screenshot of spec "Preview of a circle". Also verify that a corner drag on a 3840 × 2160 source keeps the preview live, with the time per preview frame measured.
- [x] 4.3 Move `ANTS` and the dash drawing from `ui/js/atlas/canvas.js` to `ui/js/ants.js`, so ATLAS's frame looks the same. Draw every quad's outline as ants, and the edges and handles of the active quad only (D5). Activate a quad by a click inside its shape. Verify:
  - via CDP for spec "Two signs", "Click beside a circle", "Activate by its shape" and "Corners of an inactive quad";
  - the ATLAS marquee is unchanged by a screenshot compared with before;
  - CPU use with 10 quads is recorded in `task.md`.
- [x] 4.4 Pen gestures on the active path (D6): add an anchor on a segment by splitting, remove it by a click under 3 px (keeping at least 2), Ctrl+drag an anchor or handle with the smooth mirror, Alt+drag from an anchor to pull out handles, Alt+click to make a corner, Alt+drag to break a handle, Shift for 45° steps, and the corner/anchor collision rule. Verify via CDP for every scenario of "Pen adds and removes anchors", "Pen moves anchors and handles" and "Pen converts anchors", and "Path follows the quad". Also check that each gesture is one undo step (spec "Undo a path edit").
- [x] 4.5 P drawing (D7): click adds a corner, click-drag a smooth anchor, a click on the first anchor closes it, Esc cancels without closing the app, and fewer than 2 anchors never replace the path. Verify via CDP for spec "Draw a teardrop" and "Esc while drawing", in standalone and in ShareX mode.
- [x] 4.6 Add to the help overlay the shapes, Shift+S, P, the Pen gestures and Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y in RIP, with one sentence on its RIP card. Add the same to the README. Verify by reading both against the specs.
- [x] 4.8 Pen cursors (designer's review request): Lucide `pen-tool`, `plus`, `minus`, `circle`, `chevron-up` and `mouse-pointer-2` as SVG cursors in `icons.js`. Strokes are 1 px white over a 1 px dark halo at 20 % opacity. The cursor is chosen in `editor.js` from the hover, held Ctrl/Alt and P. The cursor sheet was approved by the designer. Verify via CDP the stage cursor for every case of spec "Pen cursors", including a Ctrl/Alt press without a mouse move.
- [x] 4.7 Designer review in a test build on a copy of real photos, with `%APPDATA%\ripper` backed up and restored afterwards:
  - the shapes and popup;
  - ants and the active-only quad;
  - the Pen gestures and P;
  - undo.

  Record the outcome in `task.md`. Verify with the designer's explicit approval.

## 5. Integration and release

- [x] 5.1 End-to-end check on a release build:
  - Circle, 8-gon and Path quads on one photo;
  - Send, Copy (pasted back and checked for alpha), Save All, and the ShareX output with a `$output` path;
  - the Atlas output into a test workspace, exported with EXTEND.

  Verify the masks pixel by pixel against `outline`, that the transparent corners fill under EXTEND (spec "Atlas tile with EXTEND"), and that a Rect output is byte-identical to 2.2.0's.
- [ ] 5.2 Bump `src-tauri/Cargo.toml` to 2.3.0 and run `scripts/release.ps1 -DryRun`. Publish only after the designer's explicit yes, then update this machine through the button. Verify with the release URL and the installed version.
- [ ] 5.3 Update `task.md` (status, next step) and the project memory. Verify that both name the 2.3.0 release.

## Workflow follow-up

- Archive with `/opsx:archive add-quad-shapes` after the release. This creates the main specs `quad-shapes` and `rip-undo` and updates `quad-editing`.
