# Design

## Context

- **Quad model (UI).** A RIP quad lives in `ui/js/state.js` as `{ corners, aspectKey, custom, pot, dirty }`. `quadInfo` derives the clockwise corners, validity and output size through `geom.js`, which mirrors `src-tauri/src/geom.rs`. `geom.test.mjs` keeps the two in step.
- **Export.** `export.rs` takes a `QuadReq { corners, aspect, pot, number }` and renders it in one place, `export::render`, with `warp::warp`. Send, Copy, Save All, the ShareX output and Atlas tiles all go through it.
- **Canvas.** `editor.js` draws on a 2D overlay above the WebGL image (`view.js`):
  - every quad's edges and corner handles;
  - the draft of a new quad.

  A click on empty image starts a new quad. A click inside an inactive quad activates it. Handles of every quad can be dragged.
- **Preview.** `preview.js` maps output pixels through the homography in a WebGL2 shader. Transparency shows on the existing checker.
- **Keys.** They are handled in `main.js` `onKey`, by `e.code` so they work in any keyboard layout.
  - Esc cancels the draft, and otherwise closes the app.
  - There is no undo in RIP.
- **Marching ants.** They exist only in ATLAS: `ANTS` and `drawMarquee` in `ui/js/atlas/canvas.js`.
- **Defaults.** Aspect and power of two for new quads persist in `settings.rs` (`aspect_mode`, `aspect_custom`, `pot`).

## Goals / Non-Goals

**Goals:**
- One shape description and one outline function, mirrored in JS and Rust, so preview, canvas and export agree.
- The homography and output size code untouched. A Rectangle export must stay byte-identical.
- Pen gestures and P drawing that feel like Illustrator, inside the existing editor.
- Undo of quad edits by snapshots, simple enough to cover every edit kind.

**Non-Goals:**
- Exact analytic coverage of curves. Flattening to a fine polygon with supersampled coverage is enough.
- Changing ATLAS tiles, which stay rectangles for snapping, marquee and hit tests.

## Decisions

### D1. Shape data in the quad's unit square

Each quad gains a `shape`, sent to Rust as-is:

```
{ kind: 'rect' }
{ kind: 'circle' }
{ kind: 'ngon', n: 3..12 }
{ kind: 'path', nodes: [ { p: [u, v], hin: [u, v] | null, hout: [u, v] | null, smooth: bool } ] }
```

- **Coordinates.** `(u, v)` lie in the quad's unit square: `(0, 0)` is the first corner, which becomes the texture's top-left, `u` runs right and `v` runs down.
  - The unit square maps to the image by the existing `squareToQuad(cw)`, and to the output by `(u·w, v·h)`.
  - So a path follows the quad when corners move, and scales with the output when aspect or power of two changes (spec "Path follows the quad").
- **Handles.** `hin` and `hout` are absolute positions, and null means no handle.
  - `smooth: true` keeps both handles on one line through the anchor.
  - A node with handles and `smooth: false` is a broken corner, as after Alt+drag on a handle.
- **Anchors** are clamped to the unit square, since nothing outside it reaches the output. Handles are not clamped.

In Rust, `Shape` is a serde enum tagged by `kind` with `#[serde(default)]` = `Rect`. An old UI that sends no shape therefore still gets rectangles.

Alternative: *store paths in source-image pixels.* Rejected. The path would drift off the plane when the user corrects a corner, and the export geometry would depend on the photo rather than the plane.

### D2. One outline function, mirrored in `shape.js` and `shape.rs`

`outline(shape, w, h) -> [[x, y]]` returns a closed polygon in output pixels:

- **rect:** the 4 corners.
- **circle:** the ellipse with centre `(w/2, h/2)` and radii `w/2, h/2`. Its segment count is chosen so that the chord error stays at most 0.05 px.
- **ngon:**
  - the vertices are `θ_k = -90° + k·360°/N`, rotated by `+180°/N` for even N so the top side is flat;
  - then they are scaled per axis so that their bounding box is exactly `[0, w] × [0, h]`. This is spec "touches all four sides" and "Triangle points up".
- **path:** each cubic segment `p_i, hout_i, hin_{i+1}, p_{i+1}`, with a missing handle equal to its anchor, is flattened by recursive subdivision to 0.05 px in output space.

A shared fixture, `tests/fixtures/shapes.json`, is generated from `shape.rs` and checked by both `tests/shape_fixture.rs` and `ui/js/shape.test.mjs`, as `quads.json` does for the homography. The canvas uses the same function with `w = h = 1` and a looser tolerance. Each outline point is then mapped by `squareToQuad` for display and hit tests.

**Circle → Path** uses 4 smooth anchors at the ellipse's extremes, with handles at κ = 0.5522847498 of the radius. The radial error is 0.027 %, which is under 1/255 alpha at any pixel up to an 8192 output (spec "Circle to Path"). **N-gon → Path** gives the N vertices as corner anchors, and **Rect → Path** gives the 4 corners.

### D3. Mask in Rust after the warp

`export::render`:
1. warps exactly as now;
2. returns at once for `Rect`, so the output stays byte-identical;
3. otherwise builds a coverage mask from `outline(shape, w, h)` and multiplies each pixel's alpha by the coverage, rounding to nearest.

The new module is `mask.rs`.
- **Coverage:** 4 × 4 samples per pixel at sample centres, nonzero winding.
- **Speed:** each sample row intersects the polygon edges once; the crossings are sorted and the covered spans are accumulated.
- **Cost:** O(4h · E) for E edges. An 8192² output with a 2000-edge path stays well under a second, and typical 1–2K outputs take milliseconds.
- **Validity:** a non-Rect shape whose outline area is below 1 px² is `Invalid::EmptyShape`, "the shape has no area", and is not exported, like a degenerate quad.

Alternative: *exact area coverage, as font rasterizers do.* Rejected for now: more code for a difference of at most 1/16 alpha step on edge pixels.

### D4. Mask in the preview: a 2D canvas texture

The preview keeps its shader and samples a second texture, the mask, at `v_out / u_size`.
- The mask is drawn by Canvas2D into an offscreen canvas at the preview's pixel size. It fills the same `outline(shape, …)` polygon, scaled to that size, so the browser's anti-aliasing gives the soft edge.
- It is re-uploaded only when the shape, the output size or the preview size changes, not when corners move. A corner drag therefore costs the same as today.

Spec "Preview matches export geometry" holds, because the polygon is the same and only the edge filtering differs. The existing requirement already allows that.

### D5. Canvas: ants for every shape, the quad only for the active one

- `ANTS` and the dash drawing move from `ui/js/atlas/canvas.js` to a shared `ui/js/ants.js` with `strokeAnts(ctx, dpr, now)`. ATLAS's frame keeps the same look; this is a move, not a redesign.
- `drawOverlay` draws:
  - the ants along each quad's outline mapped to the screen;
  - for the active quad only, its blue edges and corner handles, as today, under the ants.

  An invalid quad keeps its `--danger` edges while active.
- **Path handles** of the active quad are drawn as Illustrator does:
  - an anchor is a 6 CSS px square, filled when selected-for-drag;
  - a handle is a 4 px circle on a 1 px line to its anchor;
  - colours are `--blue` with the existing white halo.
- **Animation.** While at least one quad exists and RIP is shown, `drawOverlay` requests the next frame. Only the overlay redraws, never the WebGL image. rAF pauses by itself in a hidden window, and the ATLAS mode stops the loop.

### D6. Pointer routing in `editor.js`

`onDown` tries these in order:
1. pan;
2. Pen drawing (D7);
3. for an active Path:
   - **Alt:** on a handle → break-drag; on an anchor → pull out handles, or Alt+click → corner;
   - **Ctrl:** on a handle → handle drag, mirrored when smooth; on an anchor → anchor drag;
   - **no modifier:** on an anchor → pending click (remove on release under 3 px); on a segment within 8 px → add an anchor by splitting that cubic at the nearest `t` (de Casteljau, so the curve does not change).
4. the active quad's corner handles;
5. inside any quad's **shape**, which activates that quad;
6. otherwise a new quad draft.

**Collision.** Anchors of a Rect-derived path sit exactly on the quad's corners. For a plain press on such a spot:
- moving 3 px or more drags the quad corner, as the user expects from today's handles;
- releasing under 3 px removes the anchor;
- Ctrl and Alt always act on the anchor.

**Shift** constrains the dragged point's offset from its anchor, or from its start for an anchor drag, to 45° steps in output pixels, i.e. `(u·w, v·h)`.

The pointer maps to `(u, v)` by the inverse of `squareToQuad`. A point beyond the horizon, where w ≤ 0, is ignored.

### D7. Pen drawing with P

P on an active Path quad enters `pen` mode with an empty draft:
- **Press:** a press adds an anchor.
- **Drag:** a drag of 3 px or more before release makes it smooth, with `hout` = pointer and `hin` mirrored, as with Illustrator's click-drag.
- **Close:** a press within 8 px of the first anchor closes the path, and a drag there shapes the first anchor's `hin`.
- **Commit:** closing with at least 2 anchors replaces the shape's nodes as one history step.
- **Cancel:**
  - Esc keeps the old path, and `main.js` asks `editor.cancelPen()` before its close-the-app branch;
  - so does switching quads, changing shape or loading a new image.

While drawing, the ants show the old path, and the draft is drawn in `--blue` with a rubber-band segment to the pointer.

### D8. History by snapshots (`ui/js/history.js`)

A snapshot is a deep copy of `{ quads (without dirty), active }`.
- **Recording:** `record()` is called before each change.
  - A drag records once, on its first movement.
  - A click edit, a shape or N change, the aspect or PoT of the active quad, create and remove each record once.
- **Undo:** pushes the current state to `future`, restores `past.pop()` and emits `'quads'`. Redo is the mirror.
- **Limits:** `past` is capped at 100. A new `record()` clears `future`, and `setSource` clears both.
- **Dirty:** after undo or redo, a quad is marked dirty unless an identical quad, compared by JSON, existed in the replaced state. The update guard (`unsavedCount`) therefore stays truthful.
- **Keys** in `main.js`, RIP branch: Ctrl+Z, Ctrl+Shift+Z and Ctrl+Y by `e.code`, ignored while typing, as in ATLAS.

Alternative: *inverse operations, as ATLAS does in Rust.* Rejected here: the RIP state is small JSON, and snapshots cover new edit kinds without extra code per edit.

### D9. Choosing a shape: header icon, popup, Shift+S, settings

- **Header icon.** The Quads panel header gets an `icon-btn` right of the title. It shows the active quad's shape, or the default shape when there are no quads. The icons are Lucide as published (lucide-static 1.72.0, ISC): `square`, `circle`, `octagon` and `pen-tool`, kept in `icons.js` with their source comment, as the existing icons are.
- **Popup.** Tapping the icon opens a popup under it, built from the panel tokens:
  - four rows, each with an icon and a name;
  - a number field for N on the N-gon row, 3–12, defaulting to the last N.

  Esc or a click outside closes it, and choosing a row closes it too.
- **Shift+S** (`e.code === 'KeyS'` with Shift, no Ctrl) cycles the active quad's shape.
- **Defaults.** The chosen shape and N become the defaults. `settings.rs` gains `shape: String` ("rect" by default) and `ngon: u32` (8), both with `#[serde(default)]` so old `config.toml` files load.
- **Path as default.** A new quad gets a Path started from its rectangle (D2).

## Risks / Trade-offs

- **[Risk] The constant animation of the ants costs power on a laptop.** → Only the overlay redraws, at most once per display frame, and only in RIP with quads present. rAF pauses in a hidden window. Check CPU use with 10 quads.
- **[Risk] Alt on Windows can focus a window menu.** → ATLAS already uses Alt+drag without trouble. `preventDefault` on the Alt keydown in RIP too.
- **[Risk] Hit-test ambiguity when anchors, handles and corners overlap.** → The fixed order of D6 and the 3 px click rule. Ctrl and Alt always reach the path.
- **[Risk] The JS and Rust outlines drift apart.** → Shared fixtures in both test suites, as for the homography.
- **[Trade-off] Supersampled 4 × 4 coverage** gives 17 alpha levels on edges, versus exact area. It is visually smooth at texture scale and can be refined later without changing the spec.
- **[Trade-off] The Rectangle export path is untouched** (early return), so existing outputs stay byte-identical.

## Migration Plan

- **Data:** new fields with serde defaults, and no data migration. The RIP quads are not persisted, and the settings gain two defaulted fields.
- **Release:** version 2.3.0 through `scripts/release.ps1` after the designer's yes.
- **Rollback:** reinstalling 2.2.0. Older builds ignore the extra settings fields.
