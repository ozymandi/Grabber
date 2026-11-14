# Design

## Context

ripper 2.0.0 ships the atlas core (archived change `add-texture-atlas`, main specs `atlas-workspace`, `atlas-tiles`, `atlas-export`).

**Rust (`src-tauri/src/atlas/`):**
- `model.rs` holds the manifest. A tile is an axis-aligned integer rect `x, y, w, h` that already includes the rotation, plus `rot` in quarter turns. Operations are pure methods that return `Refused` with a status text.
- `store.rs` applies every edit as lock → re-read → apply → atomic write. A ShareX process and an open window never overwrite each other. SAVE packs `manifest.json` and the referenced `items/*.png` into the `.ripper` zip and deletes unreferenced items from the working folder.
- `render.rs` renders each tile once with `warp::warp`, the RIP sampler with Jacobian-adaptive supersampling. It clips the tile per cell and composites it with straight-alpha over.
- `atlas_cmd.rs` exposes one `atlas_edit(Edit)` command. Each call returns the whole new `Doc`.

**UI (`ui/js/atlas/`):**
- `canvas.js` draws everything on one 2D canvas: sector fills, tiles via `drawImage` with `rotate(rot·90°)`, borders, labels, selection and handles. The "+" buttons are DOM elements.
- `atlas.js` owns the panels, the keys and a 500 ms revision poll.

Requirements are in the change's specs (`atlas-guides`, `atlas-undo` and the deltas for `atlas-tiles`, `atlas-export`, `atlas-workspace`). Motivation is in proposal.md.

## Goals / Non-Goals

**Goals:**
- **Lossless where it can be.** 0/90/180/270° rendering stays an exact pixel permutation, and four R presses return the exact placement, even about a moved pivot.
- **Undo never fights the two-writer model.** Every undo is an ordinary edit applied under the lock to the current manifest.
- **Performance** matches 2.0.0:
  - 60 fps while dragging with 20 tiles, guides and the grid;
  - a 4096 sector exports with EXTEND in well under a second on this machine.
- **The Rust core stays Tauri-free** and unit-testable; each new operation and its inverse get tests.

**Non-Goals:**
- **Persistent history.** It is not kept on disk and not shared between windows.
- **Perfect undo under conflicts.** When another window or ShareX touched the same objects, the conflicting part is skipped (spec), not merged.
- **Guide display at full source resolution** beyond 4096 px.
- **WebGL.** The 2D canvas stays; free rotation is `ctx.rotate`.

## Decisions

### D1. Tile frame in half-pixel integers (format 2)

```
Tile {
  id, item, w0, h0, source,          // unchanged
  c2: [i64; 2],      // centre x2, in half pixels (exact for odd sizes)
  w, h: i64,         // placed size BEFORE rotation (scaled original)
  rot: u8,           // quarter turns clockwise, 0..3
  tilt: f64,         // extra degrees, in (-45, 45]; 0 for lossless tiles
  pivot: Option<[i64; 2]>,  // whole pixels from the unrotated frame's top-left; None = centre
}
```

- **Angle.** The total is `rot·90 + tilt`.
  - R changes only `rot`.
  - A free turn sets the total and splits it again: `rot = round(total/90) mod 4`, `tilt = total - rot·90`.
  - A tilt within 1e-9 of 0 is stored as exactly 0, so Shift-snapping to 90° returns to the lossless path.
- **R about the pivot.** The pivot is `p2 = c2 + R(rot)(pivot2 - half_size2)`, all in doubled integers. R computes `c2' = p2 + R90(c2 - p2)` exactly in integers. Four turns are therefore an exact identity, whatever the pivot.
- **Free turn about the pivot.** It is the same formula in floating point, rounded to the nearest half pixel. The error is at most 0.25 px, and the result is resampled anyway.
- **Bounding box.** Lossless placement is derived from `c2`:
  - With `tilt == 0`, the box is `(c2 - rotated_size)/2`, floored to whole pixels. When the parity of the centre and the size differ, the tile snaps by half a pixel instead of being resampled. That is the same trade 2.0.0 made with its integer truncation.
  - With `tilt ≠ 0`, the box is the floor/ceil of the four rotated corners.
- **Who uses the box.** Placement, sector overlap checks, the tray, the remove-sector rule and export clipping all use the bounding box. Automatic placement only creates tiles at 0°, so it is unaffected.
- **Scaling about a corner or the pivot.** The fixed point is computed in canvas space before the change. The new `w`, `h` and `c2` are solved from it and rounded to half pixels. A moved pivot scales proportionally and rounds to whole pixels; `None` stays the centre.
- **Format upgrade.** Format 1 → 2 happens when a manifest is read:
  - `c2 = (2x + w, 2y + h)`;
  - `(w, h)` is swapped back for odd `rot`;
  - `tilt = 0`, `pivot = None`.

  So 2.0.0 tiles keep their exact pixels (spec "Document format compatibility"). `format > 2` is refused with `Error::NewerFormat`, and OPEN leaves the current workspace untouched.

*Alternative: keep `x, y, w, h` and add an angle about the rect centre.* This cannot express an exact quarter turn about an arbitrary pivot without accumulating rounding, and would break the four-turn identity in the spec.

*Alternative: f64 everywhere.* It is simpler, but 90° rendering would need epsilon checks and could drift after many edits.

### D2. Rendering tilted tiles with antialiased edges

`render_tile` keeps two paths:
- **Exact path** (`tilt == 0`): unchanged. It is a permutation when the size matches, and `warp` otherwise.
- **Tilted path:**
  - the output is the tile's bounding box;
  - the four "source corners" passed to `warp` are where the box corners land in the original under the inverse affine map, so they fall outside the original;
  - `warp` already returns transparent outside the source.

`warp` chooses sub-samples from the Jacobian. At scale 1 that is one sample per pixel, which would leave stair-stepped edges. `warp` therefore gains a `min_sub` argument: 1 for RIP and exact tiles, 4 for tilted tiles. That gives 16 coverage levels on the edges, and the same filter still handles shrinking. The cost is about 16 bilinear samples per pixel, roughly 60–120 ms for a 2048² tile with rayon, cached per export like today.

*Alternative: analytic edge coverage with a polygon rasteriser.* It is exact, but it is a second rendering path to maintain. Supersampling reuses the tested sampler.

Display in the UI is `ctx.rotate` around the derived centre with `imageSmoothingQuality = 'high'`, and selection frames and handles follow the rotated corners.

### D3. Edge fill per cell

Edge fill is a pass over each **cell** image after its tiles are composited. A cell is one sector. GROUP FILE renders cell by cell into the sheet, so a pixel never sees another cell (spec "Edge fill per sector").

1. **Seeds** are pixels with alpha ≥ 128, which picks solid tile colour over a faint fringe.
2. **Nearest seed.** An exact Euclidean nearest-seed transform runs over the cell: the separable Felzenszwalb–Huttenlocher EDT with argmin tracking, rows and columns parallel with rayon.
3. **Fill.** For every pixel with alpha < 255 whose nearest seed is within `edge` px:
   - an opaque underlay takes the seed's colour;
   - the pixel is composited over it.

   Empty pixels become the seed colour, and antialiased edges become solid without a fringe. Pixels farther than `edge` stay as they are.

The sheet as a whole is never dilated, which is why GROUP FILE equals GROUP SET pixel for pixel. Cost: two linear passes. Expect about 100 ms at 4096² and about 0.4 s at 8192².

*Alternative: iterative 3×3 dilation N times.* It is simple, but square-shaped and O(N) passes, so 16–2048 px would be slow and anisotropic.

*Alternative: jump flooding.* It is fast but approximate, and its errors show up as visible seams in a texture tool.

### D4. Guides and per-workspace settings in the manifest

```
Manifest (format 2) adds:
  guides: [ { sector: {u, v}, item: String, w: u32, h: u32 } ]   // at most one per sector
  guide_show: bool (true), guide_opacity: u8 (50, percent)
  fill: "transparent" | "extend", edge: u32 (16)
```

- **Storage.** Guide PNGs are ordinary items in `items/`, so the zip packing, unpacking and thumbnail endpoint stay one code path. The save-time cleanup and the zip writer count items referenced by tiles **or** guides.
- **LOAD** is a dedicated command, `atlas_load_guides(paths, target: Option<Sector>)`.
  1. **Classify.** Rust decodes each file's header for its size and sorts the files by name: `(?i)\.(1\d{3})\.png$` with 1001–1999 marks a set member, anything else is single. More than one single file is refused.
  2. **Write items.** The items are written before the manifest, as in 2.0.0.
  3. **Map under the lock.** Set members map to `(umin + col, vmin + row)`, where `umin, vmin` is the grid's bottom-left at that moment. That is the same origin the canvas labels use. Missing cells are created. A single file goes to `target`, otherwise to the active sector, otherwise to the first sector in UDIM order.
  4. **Report.** The result returns the counts loaded and created, a not-square count for the status warning, and the inverse edits (D6).
- **The width limit from the discussion is not needed.** A UDIM number can only address columns 0–9, so nothing is refused. Number and file name are checked instead.
- **Sector removal** also drops that sector's guide, and its inverse restores both.
- **View and export settings** (`guide_show`, `guide_opacity`, `fill`, `edge`) are written through `atlas_edit` like padding, so a second window sees them. They return no inverse and so never enter the history.
- **Display.** `ripper://atlas/<ws>/<item>?max=4096`, cached in `thumbs/` like tile thumbnails.

### D5. Canvas layers, grid, preview and gestures

Draw order inside each sector, bottom to top:
1. **Sector fill** (`--panel-2`).
2. **Graph-paper grid.**
   - Lines are computed in screen space for visible sectors only, 1 device pixel wide.
   - Major lines run every S/8 in `--line-2`, minor every S/64 in `--line`.
   - Minor lines are skipped when `S/64·zoom < 6·dpr`.
3. **Guides** with `globalAlpha = opacity` when Show is on.
4. **Tiles**, rotated.
5. **The G preview, while G is held:** for each guide, a cached offscreen copy tinted `#FF00FF` with `source-in`, at alpha 1. It is built once per guide image.
6. **Overlay:** borders, labels, selection frames on the rotated corners, handles, the rotate zone, the pivot, and the "+" DOM buttons.

**Gestures**, for a single selected tile:
- **Hit tests** are done in tile-local space: the pointer is inverse-rotated about the tile centre.
  - A handle is a 6 px square at a rotated corner.
  - The **rotate zone** lies outside the frame and within 16 CSS px of a corner.
- **Rotate drag.** The swept angle is `atan2` around the pivot. Shift rounds the total to 15°. The status shows `Rotate 37.5°`. On release, one `turn {id, angle}` edit is sent.
- **Pivot.** It is drawn with the Lucide `crosshair` geometry at 16 px in `--blue`.
  - Alt+drag snaps within 8 screen px to the nine points; otherwise it rounds to whole pixels. On release it sends `set_pivot`.
  - Alt+double-click sends `set_pivot {pivot: null}`.
  - `keydown Alt` is `preventDefault`ed so WebView2 never moves focus.
- **Scale.** The handle drag projects the pointer on the tile's own axes. Alt changes the anchor to `pivot`.
- **Hold G.** `keydown KeyG` (not repeat) sets `preview = true`; `keyup KeyG` and `window blur` clear it.
- **Rotate cursor.** A CSS `cursor: url(data:image/svg+xml…) 12 12` is built from Lucide `rotate-cw` 1.72.0, as published. CSS variables cannot reach a cursor, so the stroke uses the literal `--ink` value over a `--bg` halo for contrast, noted in `icons.js`.

**Panels:**
- **Guides** (new, between Sector and Export):
  - head with the sector name;
  - LOAD... and REMOVE action buttons;
  - the Show checkbox;
  - Opacity as the O'Tool `slider-capsule`, a range plus a number, 0–100.
- **Export** gains a Fill `seg-row` (TRANSPARENT / EXTEND) and an `Edge, px` number field, disabled with TRANSPARENT.

### D6. Undo as inverse edits computed by Rust

- **Inverses.** Every forward edit returns its inverse, built from the manifest **under the lock, before** the edit applies. `EditResult` gains `inverse: Vec<Edit>` and `label`. Inverses are fine-grained, one object per sub-edit:
  - **move, scale, turn, R, pivot:** `set_frame {id, frame}`, the full D1 frame including the pivot;
  - **Delete:** `restore_tile {index, tile}`, with the same id, item and z-order index;
  - **add or remove sector:** `remove_sectors {cells}` and `add_sectors {cells, guides}`;
  - **guide LOAD or REMOVE:** `set_guide {sector, guide: Option}` per sector, plus `remove_sectors` for created cells.
- **Applying.** `atlas_apply_steps(edits)` applies a list **best effort** in one lock and one write.
  - Each sub-edit that refuses is collected as a skipped reason ("tile 12 no longer exists", "sector 1002 kept, a tile arrived there"), and the rest still applies.
  - It returns the doc, the skipped list and the inverse of what it actually applied. That inverse becomes the redo, or the undo again.
  - Forward edits stay all-or-nothing, as in 2.0.0.
- **The history lives in `atlas.js`.**
  - Two stacks of `{label, edits}`, capped at 100. A new forward step clears redo.
  - NEW and OPEN clear both.
  - ShareX and RIP adds never go through `atlas_edit`, so they never enter the history (spec).
  - With nothing to undo, the status says so.
- **Originals stay available.** AppState keeps `held: HashSet<item>` for items that leave the manifest while still referenced by this window's history (Delete, guide replace or remove). `save_as` skips `held` items in its cleanup but does not pack them. NEW and OPEN clear `held`. Items are reclaimed by the first save after the window closes.
- **Unsaved mark.** Undo and redo are normal writes, so `revision` changes and the unsaved mark appears (spec).

*Alternative: snapshot undo (restore the whole manifest).* It is trivial, but it would delete tiles a ShareX capture added in between, which violates the spec. Rejected.

*Alternative: JS computes inverses from its copy of the doc.* The copy may be stale by up to 500 ms (poll), which would make a wrong inverse. Rust sees the true state under the lock.

### D7. Version and release

`Cargo.toml` → 2.1.0. `scripts/release.ps1` publishes it after the designer's explicit yes. As in 2.0.0, `icons/icons.ai` is temporarily marked `assume-unchanged` and unmarked afterwards, never committed.

## Risks / Trade-offs

- **[A 2.0.0 copy opens a format-2 file]** 2.0.0 ignores unknown fields and would misplace tilted tiles. → The updater moves installs forward; the README notes that 2.1 files need 2.1. 2.0.0 itself cannot be changed.
- **[Half-pixel snap on a quarter turn]** A tile with an odd size difference shifts by half a pixel when turned 90° (D1). → This is lossless in pixels and reversible by four turns; 2.0.0 behaved the same.
- **[Tilted tile export cost]** A minimum of 4×4 sub-samples on large tilted tiles. → Exact tiles keep `min_sub = 1`. Export timing is measured in tasks against the 2.0.0 figures.
- **[Another window saves and purges a held item]** That window does not know this window's history. → Undo of that Delete is skipped with "original is gone" (spec "Conflicts are skipped"); the `.ripper` file still holds the last saved state.
- **[Memory for the guide preview]** One tinted 4096² offscreen canvas per guide is 64 MB each. → They are built lazily, only for guides in visible sectors, and dropped when the preview ends.
- **[Magenta is not an O'Tool token]** → Named to the designer as a deviation and recorded in `task.md`.

## Migration Plan

- **Working folders and `.ripper` files** are upgraded from format 1 to 2 when read (D1). Nothing is rewritten until the next edit or save.
- **`config.toml`** is unchanged. All new settings live in the workspace.
- **Rollback.** Reinstall 2.0.0 and open only files last saved by 2.0.0. Format-2 files need 2.1.0 (Risks).

## Open Questions

- **Grid line colours** (`--line-2` / `--line`) and the pivot size may be tuned on the designer's visual review (task 6.x) without changing the specs or tasks.
