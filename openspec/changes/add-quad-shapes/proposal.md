# Proposal

## Why

Many surfaces worth ripping are not rectangles, for example a round road sign, an octagonal STOP sign or a decal with a curved edge. Today the user flattens the quad around such a shape and then cuts the shape out in another program. The quad already gives an exact perspective plane, so the cut-out can happen in ripper itself as a mask on that plane. A circle seen in perspective is exactly the ellipse inscribed in the quad around it.

## What Changes

- **Shapes: a quad is the plane, its shape is a mask inside it.** The homography, output size, aspect and power of two stay as they are. Outside the shape the output is transparent, and the shape's edge is anti-aliased. Every quad has one of four shapes:
  - **Rectangle:** the whole quad, as today, and the default.
  - **Circle:** the ellipse inscribed in the quad.
  - **N-gon:** a regular polygon, N from 3 to 12, that touches all four sides of the quad. N = 3 points up, and even N has a flat top side, like a STOP sign.
  - **Path:** a closed Bézier path in the quad's own coordinates, edited like Illustrator's Pen tool (below).
- **Choosing a shape:**
  - an icon in the header of the Quads panel shows the active quad's shape;
  - tapping it opens a popup with the four shapes, and an N field for N-gon;
  - **Shift+S** cycles Rectangle → Circle → N-gon → Path for the active quad;
  - new quads take the last shape chosen, and the shape and N are kept in `config.toml`, like Aspect.
- **Canvas:**
  - every quad's shape outline is drawn as marching ants, as in the ATLAS selection frame;
  - the quad itself (edges and corner handles) shows only for the active quad;
  - a click inside an inactive quad's shape makes it active.
- **Path editing (Pen-tool gestures on the active quad's path):**
  - click on a segment adds an anchor without changing the curve;
  - click on an anchor removes it (minimum 2 anchors);
  - Ctrl+drag moves an anchor or a handle;
  - Alt+drag from an anchor pulls out symmetric handles, making the anchor smooth;
  - Alt+click on a smooth anchor makes it a corner again;
  - Alt+drag on a handle breaks it from its partner;
  - Shift keeps a drag at 45° steps in the quad's plane;
  - **P** redraws the path with the Pen: click adds a corner anchor, click-drag adds a smooth one, and a click on the first anchor closes the path. Esc while drawing restores the previous path.
  - Switching a quad to Path starts from its current shape: a rectangle gives 4 corner anchors, a circle 4 smooth anchors, an N-gon N corner anchors.
- **Undo in RIP:**
  - Ctrl+Z undoes, and Ctrl+Shift+Z and Ctrl+Y redo, every change to the quads: creating and removing a quad, moving corners, shape and N, path edits, and the quad's aspect and power of two;
  - a new source image (OPEN, PASTE, drop or ShareX launch) clears the history;
  - exports, folders and settings are not undone.
- **Outputs:** Preview, Send, Copy (PNG with alpha), Save All, the ShareX output and Atlas tiles all carry the mask. In ATLAS, EXTEND already fills transparent pixels from the tile's edge, so a round tile's corners fill too. Tiles stay rectangles for snapping and selection.
- **Help and README** list the shapes, Shift+S, P, the Pen gestures and Ctrl+Z in RIP.
- **Version 2.3.0.**

Out of scope:
- shapes in ATLAS (tiles keep their rectangle);
- holes, several paths per quad and open paths;
- a shape that sets the perspective by itself (for example fitting a regular N-gon to N clicked corners);
- rotating an N-gon inside the quad;
- undo of settings, folder choices or written files.

## Capabilities

### New Capabilities

- `quad-shapes`: the shape of a quad and its mask. Covers the four shapes and their geometry, choosing a shape, marching-ants display, Path editing with Pen-tool gestures, and the mask in every output.
- `rip-undo`: undo and redo of quad edits in RIP mode, and when the history is cleared.

### Modified Capabilities

- `quad-editing`:
  - corner handles show and drag only on the active quad;
  - a click inside a quad's shape, not its whole quad, activates it;
  - the preview shows the masked output.

## Impact

- **UI:**
  - `ui/js/state.js` — shape on each quad and in the defaults, history;
  - `ui/js/geom.js` — shape outlines and the Bézier helpers;
  - `ui/js/editor.js` — ants, active-only quad, hit tests, Pen gestures, P drawing;
  - `ui/js/preview.js` — the mask;
  - a new shape-popup module and the Quads panel header;
  - `ui/js/main.js` — Shift+S, P, Ctrl+Z, Esc while drawing;
  - `ui/index.html` — the help.
- **Rust:**
  - `export.rs` — `QuadReq` gains a `shape`; `render` applies the mask;
  - `geom.rs` — the shape outlines, the same as `geom.js`;
  - a mask rasterizer;
  - `settings.rs` — default shape and N.
- **Atlas:** no code change. Tiles come from `export::render` and so carry the mask.
- **Tests:**
  - Rust unit tests for the outlines and the mask;
  - node tests for `geom.js` shapes and the history;
  - CDP checks in the app.
- **Release:** version 2.3.0 in `src-tauri/Cargo.toml`.
