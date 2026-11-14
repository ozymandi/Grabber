# Tasks

## 1. Hit test (pure JS, with tests)

- [x] 1.1 Create `ui/js/atlas/marquee.js` with `touches(corners, box)` (design D3): separating axes on the box's x/y and the tile's two edge normals, touching edges count. Verify with `ui/js/atlas/marquee.test.mjs` (`node ui/js/atlas/marquee.test.mjs`, exit 0):
  - an unrotated tile: frame inside, frame over one corner, frame flush with an edge (touches), frame 1 px away (does not touch);
  - a 45° tile: frame over its bounding-box corner but off the tile (spec "Rotated outline, not the bounding box") does not touch, and frame over its tip does;
  - a frame drawn right-to-left / bottom-to-top gives the same result.

## 2. Canvas wiring (UI)

- [x] 2.1 In `canvas.js`, turn the empty-canvas press into a pending `marquee` drag (D1). Move today's click code unchanged into `clickEmpty(sec, shift)` and run it from `onUp` when the drag never started. Verify in the app via CDP:
  - spec "Click selects a sector": the sector becomes active;
  - "Small jitter is still a click";
  - Shift+click still toggles a sector;
  - pan with Space or the middle button is unaffected.
- [x] 2.2 Recompute the selection from `base` on every pointer event once the drag has started (D2), using `touches` on every tile, and call `hooks.select()`. Verify in the app via CDP:
  - spec "Frame across two tiles", including that the sector selection and active sector are unchanged;
  - "Touching is enough";
  - "Rotated outline, not the bounding box";
  - "Frame replaces the selection" and "Shift adds to the selection";
  - "Live selection";
  - "Frame from the tray".
- [x] 2.3 Add `cancelMarquee()` and call it first from the Esc branch in `atlas.js` (D4). Verify via CDP for spec "Esc cancels the frame" and that Esc with no frame still clears the selection.
- [x] 2.4 Draw the frame (D5) as marching ants: a white line with black 4 px dashes over it, 1 CSS px, no fill, moving 1 px every 50 ms and still moving while the pointer rests, only while the drag has started. Verify with a screenshot during a frame drag and after release (spec "Frame disappears on release"). Also verify spec "No undo step": a move, then a frame, then Ctrl+Z undoes the move.
- [x] 2.5 Add "Drag on empty canvas — Select by frame" and "Shift+drag — Add to the selection" to the help overlay's key list and one sentence to its ATLAS card in `ui/index.html`, and the same to the README Atlas section. Verify by reading both against the spec.
- [x] 2.6 Designer review of the frame's feel and look in a test build, on a separate test workspace with `%APPDATA%\ripper` backed up and restored afterwards. Record the outcome in `task.md`. Verify with the designer's explicit approval.

## 3. Integration

- [x] 3.1 Confirm that `add-atlas-snapping` task 3.1 (end-to-end check on the release build) also covers the marquee: select a packed row by frame, then move it with snapping, and undo. Verify that the shared check passes before the 2.2.0 release in `add-atlas-snapping` task 3.2.

## Workflow follow-up

- Release 2.2.0 through `add-atlas-snapping` task 3.2. That task has no separate version bump.
- Archive this change with `/opsx:archive add-atlas-marquee` after the release. This adds the requirements "Marquee selection", "Click on empty canvas is unchanged" and "Marquee look" to the main spec `atlas-tiles`.
