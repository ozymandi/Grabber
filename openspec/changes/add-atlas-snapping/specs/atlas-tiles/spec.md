## ADDED Requirements

### Requirement: Snap while moving
While a selection is dragged in ATLAS mode, the edges of its bounding box SHALL snap to targets within 8 screen pixels, at any zoom. The targets are sector borders, edges of unselected tiles (also in the tray) and grid lines currently drawn. Each axis SHALL snap on its own to at most one target. A sector border or tile edge SHALL win over a grid line; otherwise the nearest SHALL win.

#### Scenario: Flush to a sector border
- **GIVEN** a 1024 sector 1001 at 100 % zoom and padding 8
- **WHEN** the user drags a tile so that its left edge would land 3 pixels right of the sector's left border
- **THEN** the left edge lands exactly on the border

#### Scenario: Tile edge wins over the grid
- **GIVEN** a 1024 sector at 100 % zoom, where minor grid lines lie every 16 pixels, padding 0, and an unselected tile whose right edge is at x 300
- **WHEN** the user drags a tile so that its left edge would land at x 303, one pixel from the grid line at x 304
- **THEN** the left edge lands at x 300

#### Scenario: Hidden minor lines do not attract
- **GIVEN** a 1024 sector zoomed out so that only major grid lines are drawn
- **WHEN** the user drags a tile edge to within one screen pixel of where a hidden minor line lies, with no other target in reach
- **THEN** the edge does not snap

#### Scenario: Threshold follows the zoom
- **GIVEN** padding 0 and two tiles in the tray, the unselected one with its right edge at x 2000
- **WHEN** the user drags the other so that its left edge would land at x 2014, first at 50 % zoom and then at 200 % zoom
- **THEN** at 50 % zoom it lands at x 2000, and at 200 % zoom it lands at x 2014

#### Scenario: Axes snap on their own
- **WHEN** a drag brings a tile's left edge within the threshold of a sector border while no horizontal edge is near any target
- **THEN** only the horizontal position snaps and the vertical position follows the pointer

#### Scenario: Several tiles move as one box
- **WHEN** the user drags three selected tiles
- **THEN** the edges of the bounding box around all three snap, and none of the three is a target for the others

#### Scenario: Turned tile
- **WHEN** the user drags a tile turned 45 degrees toward a sector border
- **THEN** its bounding box, not its turned outline, lands flush with the border

#### Scenario: One undo step
- **WHEN** the user makes a snapped drag and presses Ctrl+Z
- **THEN** the tiles return to where they were before the drag in one step

#### Scenario: Scaling does not snap
- **WHEN** the user drags a corner handle so that the tile's edge passes within 2 pixels of a sector border
- **THEN** the tile scales exactly as without snapping

### Requirement: Snap at the padding
A sector border SHALL attract a moved edge both flush and at the workspace padding inside the sector. An unselected tile's edge SHALL attract both flush and at the padding outside the tile. A grid line SHALL attract flush only. With padding 0, only flush remains.

#### Scenario: At the padding from a sector border
- **GIVEN** a 1024 sector 1001 at 100 % zoom and padding 8
- **WHEN** the user drags a tile so that its left edge would land 10 pixels right of the sector's left border
- **THEN** the left edge lands exactly 8 pixels right of the border

#### Scenario: Beside another tile at the padding
- **GIVEN** padding 8, 100 % zoom and an unselected tile whose right edge is at x 300
- **WHEN** the user drags a tile so that its left edge would land at x 310, with the tiles overlapping vertically
- **THEN** the left edge lands at x 308

### Requirement: Ctrl moves without snapping
While Ctrl is held during a drag, the selection SHALL follow the pointer without snapping. Pressing or releasing Ctrl during a drag SHALL take effect at once, without moving the pointer. The help overlay SHALL list Ctrl+drag as moving without snapping.

#### Scenario: Ctrl turns snapping off
- **GIVEN** a drag in which the tile is snapped to a sector border
- **WHEN** the user presses Ctrl without moving the pointer
- **THEN** the tile jumps to the unsnapped position under the pointer and the snap line disappears; releasing Ctrl snaps it back

### Requirement: Snap lines
While an axis is snapped, the canvas SHALL show a 1 screen-pixel line in the design system's `--danger` colour along the target. The line SHALL span both the target and the dragged bounding box. For a sector border or grid line, the target is its sector. The line SHALL disappear when the drag ends or snapping is turned off. Snapping SHALL add no status text and no distance labels.

#### Scenario: Line beside a tile
- **GIVEN** padding 8 and an unselected tile whose right edge is at x 300
- **WHEN** a drag snaps a tile's left edge to x 308
- **THEN** a vertical snap line runs along x 300 over the height of both tiles

#### Scenario: One line per snapped axis
- **WHEN** only the horizontal position is snapped
- **THEN** only a vertical snap line is shown

#### Scenario: Snap lines end with the drag
- **WHEN** the user releases a snapped drag
- **THEN** the tile stays at the snapped position and the snap line disappears
