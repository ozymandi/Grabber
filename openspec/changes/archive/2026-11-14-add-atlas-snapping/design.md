# Design

## Context

The move drag lives entirely in `ui/js/atlas/canvas.js`:
- `onDown` starts `drag = { kind: 'move', start, dx, dy }`;
- `onMove` sets `dx, dy` to the pointer offset, rounded to whole pixels;
- `shown` previews every selected tile with `F.moved(t, dx, dy)`;
- `onUp` sends one `atlas_edit { op: 'move', ids, dx, dy }`. That is one undo step (archived change `add-atlas-fine-work`, D6).

Geometry already available:
- `F.rect(t)` in `frame.js` gives a tile's bounding box in whole pixels, exactly as Rust computes it. For a turned tile it is the floor/ceil of its corners.
- `sectorRect(s, size)` places sectors on whole multiples of the sector size.
- Sector sizes are 512–8192, so grid lines (multiples of size/64) and padding are whole pixels too.
- `drawGrid` decides minor-line visibility with `(S / 64) * k >= MINOR_MIN * dpr`.
- Pivot snapping uses an 8 CSS px threshold scaled by `devicePixelRatio`.

Requirements are in `specs/atlas-tiles/spec.md` (Snap while moving); motivation is in proposal.md.

## Goals / Non-Goals

**Goals:**
- **Exact results.** A snapped edge lands exactly on its target, in whole pixels.
- **Testable core.** The snap computation is pure and covered by a node test, like `geom.test.mjs`.
- **No backend change.** The edit sent on release is the same `move { ids, dx, dy }`.
- **Performance.** The drag stays at 60 fps with 20 tiles and a few sectors; snapping does no per-frame allocation proportional to the grid.

**Non-Goals:**
- Spatial indexing. Target counts are small: 4 edges per tile and sector, and grid lines are computed arithmetically.
- Snapping the turned outline of a tile. The bounding box is what placement and export use.

## Decisions

### D1. Snap the raw offset per axis, in canvas pixels

On each move:
1. Compute the raw offset `rx, ry` (pointer delta, rounded as today).
2. Compute the moving box: the union of `F.rect` of the selected tiles at drag start, shifted by `rx, ry`.
3. For each axis, the moving edges are the box's low and high edges (`x0, x1` or `y0, y1`). Their candidate corrections are `target - edge` for every target value on that axis.
4. Keep the corrections whose screen distance `|corr| * k` is ≤ `8 * dpr`:
   - among edge targets (sector borders and tile edges, with their padding offsets), take the one with the smallest `|corr|`;
   - only if there is none, take the nearest grid line.
5. Set `dx = rx + corr`, likewise `dy`. Because every target and every box edge is a whole pixel, `corr` is an integer and the result is exact.

The box is taken at drag start, so it is computed once. `F.rect(F.moved(t, d, 0))` equals `F.rect(t)` shifted by `d` for integer `d`, including turned tiles: `c2` moves by `2d` and floor/ceil commute with integer shifts.

**Alternative considered:** snapping the pointer position instead of the box edges. Rejected: the grab point inside the tile is arbitrary, so edges would not line up.

### D2. Targets per axis

The targets are built once at drag start, per axis, as edge targets `{ value, at, span }` plus each sector's extent for grid lines. `value` is where a box edge lands, `at` is the target edge itself (where the snap line is drawn, so a padded snap shows the line on the neighbour's edge, not on the moved tile), and `span` is the target's extent on the other axis.
- **Sector borders.** Each sector contributes `x0, x1` (and `y0, y1`). Each value is also offered at `x0 + p` and `x1 - p`, the padding inside the sector. The span is the sector.
- **Unselected tiles.** Each tile's `F.rect` contributes `x0, x1`, plus `x0 - p` and `x1 + p` (the padding outside the tile). The span is the tile's box.
- **Padding 0.** The padded values coincide with the flush ones and are deduplicated.

Grid lines are not stored. For a moving edge at `e`, the nearest candidate is `round(e / step) * step`, with `step = S/64` if minor lines are visible at the current zoom, else `S/8`. It counts only if that line lies within a sector's extent on that axis. Visibility uses the same formula as `drawGrid`, factored into one shared helper so the two cannot drift.

**Why sector-inner and tile-outer padding only:** the purpose is to keep the padding as clearance, as automatic placement does. A tile placed `p` outside a sector border would sit in the neighbour's area, where the neighbour's own inner padding already provides the target.

### D3. Ctrl handling

- A move does not start (and so does not snap) until the pointer has moved 3 px from where it was pressed, as the pivot drag does: otherwise a plain click on a tile near a border would snap it away.

- `onMove` reads `e.ctrlKey`.
- The canvas also listens to `keydown`/`keyup` for `ControlLeft`/`ControlRight` (by `e.code`) while a move drag is active. On a key event it recomputes the drag from the last pointer position, so the tile jumps without pointer movement.
- If focus is lost during a drag, the next pointer event's `ctrlKey` resynchronises the state.
- Ctrl during a drag does not reach the global key handler: no Ctrl+letter combination fires from Ctrl alone.

### D4. Drawing snap lines

- The drag state keeps up to two snap results `{ axis, value, span }`.
- After tiles and the selection frame are drawn, each is stroked as a 1 CSS px line (`dpr` device pixels, like the grid) in `--danger`. The line runs at `at`, from `min(span.lo, box.lo)` to `max(span.hi, box.hi)` on the other axis.
- `--danger` joins the colours `canvas.js` already reads from CSS.
- The line is drawn only while `drag.kind === 'move'`, so release and Ctrl clear it simply by recomputing.

### D5. Module layout

- A new `ui/js/atlas/snap.js` holds:
  - `targets(manifest, selectedIds)`;
  - `gridStep(size, k, dpr)`, the shared helper also used by `drawGrid`;
  - `snapAxis(lo, hi, cross, axis, step, limit)`, which returns `{ corr, at, span }` or null, and `snapMove`, which applies it to both axes.

  It has no DOM access.
- A new `ui/js/atlas/snap.test.mjs` runs with `node`. It covers the spec scenarios at the number level: flush, padding, edge over grid, hidden minor lines, zoom threshold, turned-tile box, several tiles as one box, padding 0.

## Risks / Trade-offs

- **[Risk] Dense minor grid at high zoom.** With minor lines 6–16 screen pixels apart, the selection always snaps to some line and moves in grid steps. → This is accepted as grid behaviour (designer, explore 2026-11-14). Ctrl gives free movement, and edges still win over the grid.
- **[Risk] Snapping onto an overlapping position.** A tile can snap flush into another tile's area. → Snapping only adjusts the drop; overlap was already allowed and is still the user's call.
- **[Trade-off] Turned tiles use their bounding box.** A turned tile's visible corner sits up to 1 px inside its box edge. → It matches placement and export, which also use the box.
- **[Risk] Ctrl+drag conflicts with a future Ctrl+click.** → Today Ctrl is unused on the canvas, and the help documents Ctrl+drag. Any later Ctrl gesture must be checked against it.

## Migration Plan

- **Rollout.** Front-end only. Bump the version to 2.2.0 in `src-tauri/Cargo.toml`, build, and publish with `scripts/release.ps1` after the designer's yes. Installed copies update through the updater.
- **Compatibility.** No workspace or `.ripper` format change, so 2.1.x and 2.2.0 read each other's files.
- **Rollback.** Publish the previous build as a newer version. No data migration exists to undo.
