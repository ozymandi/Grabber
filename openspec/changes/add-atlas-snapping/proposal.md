# Proposal

## Why

In ATLAS the user packs tiles by hand: flush against sector borders, side by side, or with the same gap that automatic placement keeps. Today a drag moves tiles freely to the pixel, so lining up edges and keeping an even padding is done by eye and zoom. Uneven gaps waste atlas space and leave too little room for EXTEND edge fill, so neighbouring tile colours can bleed into each other.

2.1.x added the graph-paper grid and padding but deliberately left snapping out. This change adds it now that both exist to snap to.

## What Changes

- **Snapping while moving tiles.** While the user drags a selection in ATLAS, the edges of the selection's bounding box snap to nearby targets:
  - **Targets:**
    - sector borders, outer and between sectors;
    - edges of unselected tiles, in sectors and in the tray;
    - the graph-paper grid lines that are currently drawn. Hidden minor lines do not attract.
  - **Gap:**
    - sector borders and tile edges attract at two distances: flush, and at the workspace padding;
    - grid lines attract flush only;
    - with padding 0, only flush remains.
  - **Threshold:** 8 screen pixels at any zoom.
  - **Priority:** a sector border or tile edge wins over a grid line within the threshold. Otherwise the nearest candidate wins.
  - **Axes:** each axis snaps on its own.
- **Ctrl turns snapping off** for as long as it is held during a drag. Pressing or releasing it takes effect at once, without moving the mouse. There is no persistent Snap toggle.
- **Snap lines.** While snapped, a 1 px line in the design system's `--danger` colour runs along the target edge, at most one per axis. It spans the target and the dragged box, and disappears on release or with Ctrl. It has no distance labels and adds no status text.
- **Help.** The help overlay lists "Ctrl+drag — move without snapping".
- **Undo is unchanged.** A snapped drag is the same single `move` step as before.
- **Version 2.2.0.** It is published with `scripts/release.ps1` after the designer's yes and reaches installed copies through the updater.

Out of scope:
- snapping while scaling, rotating or moving the pivot;
- snapping to tile centres or sector centres, and distance labels;
- a persistent Snap on/off setting;
- snapping for arrow-key nudges (ATLAS has none).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `atlas-tiles`: adds a requirement for snapping while moving tiles, covering:
  - targets, gap and threshold;
  - priority between edges and the grid;
  - Ctrl to disable;
  - snap lines;
  - the help entry.

## Impact

- **UI only:**
  - `ui/js/atlas/canvas.js` — move drag, Ctrl tracking, drawing the snap lines;
  - a new pure module for the snap computation;
  - `ui/index.html` — the help entry.
- **No change** to the Rust core, the `atlas_edit` commands, the manifest or `.ripper` format, export, or undo.
- **Tests:** a node test for the pure snap module, alongside the existing `ui/js/geom.test.mjs`.
- **Release:** version 2.2.0 in `src-tauri/Cargo.toml`, the single source `scripts/release.ps1` reads.
