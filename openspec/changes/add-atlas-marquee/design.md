# Design

## Context

- **Selection.** It lives in `ui/js/atlas/canvas.js`: `selTiles` (a Set of ids) and `selSectors`. `hooks.select()` tells `atlas.js` to re-render the panel.
- **Empty-canvas press.** In `onDown`, a press on empty canvas acts at once:
  - inside a sector it selects that sector, clears the tiles and sends `set_active`;
  - with Shift it toggles the sector;
  - outside sectors it clears everything.
- **Drags.** Each drag is a `this.drag` object with a `kind`, such as `move`, `scale`, `rotate`, `pivot` or `pan`. `onMove` and `onUp` branch on that kind. The `move` and `pivot` drags already wait for a 3 px start, as `move` does since `add-atlas-snapping`.
- **Esc.** It is handled in `atlas.js` and clears all selection.
- **Hit testing.** It uses `F.contains(tile, p)` on the rotated outline. `F.corners(tile)` returns the 4 outline corners in canvas space.

## Goals / Non-Goals

**Goals:**
- One new drag kind, `marquee`, that fits the existing drag pattern.
- An exact hit test between the frame and each tile's rotated outline, pure and covered by a node test.
- Today's click on empty canvas behaves exactly as before.

**Non-Goals:**
- Any change to how tiles are hit by a click, or to the move, scale, rotate or pivot drags.
- Spatial indexing. Workspaces hold tens of tiles, so testing all of them per pointer event is cheap.

## Decisions

### D1. The empty-canvas press becomes a pending `marquee` drag

`onDown` on empty canvas no longer acts at once. It stores:

```
{ kind: 'marquee', s0, last, started: false, add: e.shiftKey, base: Set(selTiles), sec, shift: e.shiftKey }
```

`sec` is the sector under the press, or null.
- **Moved 3 × dpr device px:** `started` turns true and the frame is live.
- **Released before that:** `onUp` runs today's click code with the stored `sec` and `shift`. That code is moved unchanged into a `clickEmpty(sec, shift)` method.

Alternatives:
- *Select the sector on press and undo that when a frame starts.* Rejected: it would send a `set_active` edit, which writes the working copy, and then have to revert it.
- *Use a modifier, for example Ctrl+drag, for the frame.* Rejected: the designer chose a plain drag, as in Figma, and Ctrl already turns snapping off.

### D2. The selection is recomputed from `base` on every pointer event

While `started`, each `onMove` computes the frame in canvas space from `s0` and `last` and sets:

```
hit = tiles whose outline touches the frame
selTiles = add ? base ∪ hit : hit
```

Then it calls `hooks.select()` and `request()`.
- Because the result is recomputed from `base` each time, a tile the frame leaves drops back out (spec "Live selection").
- `selSectors` and the active sector are never touched.

### D3. Hit test: separating axes on two convex quads

A new pure module, `ui/js/atlas/marquee.js`, exports `touches(corners, box)`.
- `corners` comes from `F.corners(tile)`; `box` is `{x0, y0, x1, y1}`.
- It tests the separating axes of both shapes: the box's x and y, and the tile's two edge normals.
- The shapes touch when no axis separates them. Touching edges count, so `<=` is used.
- An unrotated tile reduces to a plain rectangle overlap.

Alternative: *test each tile's bounding box.* Rejected by the spec ("Rotated outline, not the bounding box"). A click already hits the rotated outline, so the frame should match it.

### D4. Esc cancels through the canvas

`atlas.js`'s Esc branch first asks `canvas.cancelMarquee()`. If a frame drag is active, that call:
- restores `selTiles = base`;
- clears `this.drag`;
- redraws, and returns true.

Esc then does nothing else. Otherwise Esc clears the selection as before. After a cancel, the pointer release finds no drag and does nothing.

### D5. Drawing

`draw()` strokes the frame after the tiles and selection outlines, in screen space, as marching ants (the designer's revision of 2026-11-14, replacing the first `--blue` line with a fill):
- a solid white stroke, then black dashes over it with `setLineDash([4·dpr, 4·dpr])`, `lineWidth = dpr`, at `Math.round(screen) + 0.5` like the snap lines; no fill;
- `lineDashOffset` advances 1 CSS px every 50 ms from `performance.now()`, so the ants move at a steady speed whatever the frame rate;
- while the frame is up, `drawMarquee` calls `request()` again, so the ants keep marching when the pointer rests. The loop stops by itself when the drag ends.

Black and white live in one `ANTS` constant with a comment naming the deviation, like `MAGENTA` for the guide preview. The 50 ms step is set by eye, not measured from Photoshop.

The frame is drawn only while `started`.

## Risks / Trade-offs

- **[Risk] Moving the empty-canvas click to `onUp` changes its timing.** The sector now highlights on release, not on press. → This is invisible in practice. The click code itself is unchanged, and the spec scenarios "Click selects a sector" and "Small jitter is still a click" check it.
- **[Risk] `hooks.select()` on every pointer event re-renders the panel often.** → The panel render is cheap, and drawing is already coalesced per frame by `request()`. Verify there is no stutter while dragging a frame over 10 tiles.
- **[Risk] A release outside the window.** `onUp` is on `window`, so it still ends the drag. A `pointercancel` would leave `drag` set. → It is handled the same way as for the other drags; there is nothing new here.
- **[Trade-off] Touching is enough.** In a dense atlas a frame grabs edge-touching neighbours. The designer chose touching over "fully inside", and Shift+click can still remove a tile.

## Migration Plan

UI only, with no data changes. Rollback is reverting the commit. Shipped in 2.2.0 together with `add-atlas-snapping`.
