# Proposal

## Why

In ATLAS, selecting several tiles today means Shift+clicking each one. Packing a row or a whole sector at once, which snapping now makes worth doing, takes a click per tile. A marquee (a selection frame dragged across the canvas) selects them in one gesture, as in any design tool.

## What Changes

- **Marquee selection.** Dragging from empty canvas draws a frame and selects the tiles it touches:
  - **Start:**
    - the drag starts on empty canvas, inside a sector or outside;
    - the frame appears only after the pointer moves 3 screen pixels;
    - a plain click keeps today's behaviour: it selects the sector under it, or clears the selection outside sectors. Shift+click still toggles a sector.
  - **Hit rule:** a tile is selected when the frame touches its rotated outline, not just its bounding box.
  - **Shift:** a frame drawn with Shift adds the touched tiles to the selection. Without Shift the frame replaces the tile selection.
  - **Tiles only:** the frame selects tiles. It does not change which sectors are selected or which sector is active.
  - **Live:** tiles light up as selected while the frame moves, before release.
  - **Esc** during the drag cancels the frame and restores the selection it started from.
- **Look:** marching ants, as in Photoshop:
  - a 1 px line of alternating black and white 4 px dashes, no fill;
  - the dashes keep moving while the frame is up;
  - only on the frame; the outline of selected tiles stays the static `--blue` one;
  - black and white are not O'Tool tokens; this is a named deviation, approved by the designer 2026-11-14.
- **Help.** The help overlay lists "Drag on empty canvas — select by frame" and "Shift+drag — add to the selection". README gets the same line.
- **Undo is unchanged.** Selection is not an edit, so a marquee adds no undo step.
- **Release.** Published as part of 2.2.0 together with `add-atlas-snapping`. The end-to-end check and the release run once for both changes.

Out of scope:
- a frame that selects only tiles fully inside it;
- Alt or Ctrl variants that subtract from the selection;
- selecting sectors with the frame;
- Ctrl+A or other select-all shortcuts;
- auto-scrolling the view when the frame reaches the canvas edge.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `atlas-tiles`: adds marquee selection to how tiles are selected, covering the start, the hit rule, Shift, live highlighting, Esc and the frame's look.

## Impact

- **UI only:**
  - `ui/js/atlas/canvas.js` — a new `marquee` drag in `onDown`/`onMove`/`onUp`, a deferred sector click, drawing the frame, Esc;
  - a new pure module for the frame/outline hit test;
  - `ui/index.html` — the help entries;
  - `README.md` — the Atlas section.
- **No change** to the Rust core, `atlas_edit`, the manifest or `.ripper` format, export, or undo.
- **Tests:** a node test for the hit module, next to `snap.test.mjs`.
- **Release:** shares version 2.2.0 and the release tasks with `add-atlas-snapping`.
