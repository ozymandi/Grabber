# atlas-tiles Specification

## Purpose

Defines tiles, the textures placed on an atlas workspace: where a new tile lands, how it is kept at full quality, and how the user moves, scales, rotates and removes it.

## Requirements

### Requirement: Tile keeps its original
Every tile SHALL keep the full-size image it was created from, in the workspace, as the bottom state of its picture history. The tile shows its current picture state, which is the original until an LLM client writes another. The tile's size on the canvas is a scale of the current picture. Every resize and every export SHALL be rendered from the current full-size picture, never from an earlier rendered copy.

#### Scenario: Shrink and enlarge again
- **WHEN** a 1600 x 900 tile is scaled down to 200 x 113 and later scaled back to 1600 x 900
- **THEN** the exported tile matches the original pixel for pixel

#### Scenario: Export after an LLM edit
- **WHEN** an LLM client writes a new picture for a tile and the user exports its sector
- **THEN** the sector shows the new picture in the tile's frame, and going back to the original in History brings the original back into the next export

### Requirement: Automatic placement in the active sector
A new tile SHALL be placed at full size into free space of the active sector. It keeps the workspace padding from other tiles and from the sector edges. Tiles already in the workspace SHALL NOT move. If the active sector has no room, the other sectors SHALL be tried in UDIM order. Automatic placement SHALL never put a tile across a sector border.

#### Scenario: Tile fits in the active sector
- **WHEN** the active 2048 sector holds one 1024 x 1024 tile and a 512 x 512 tile arrives
- **THEN** the new tile lies inside that sector, does not overlap the first tile, keeps the padding, and the first tile has not moved

#### Scenario: Active sector full
- **WHEN** the active sector has no room for the new tile but sector 1002 does
- **THEN** the tile is placed in sector 1002

### Requirement: Oversized tile is fitted down
If a new tile is larger than a sector minus twice the padding in either direction, it SHALL be scaled down uniformly, keeping its aspect ratio, until it fits. The original SHALL stay at full size. Placement then follows the automatic-placement rule.

#### Scenario: Large capture into a small sector
- **WHEN** a 1500 x 900 tile arrives in a workspace with 1024 sectors and 8 px padding
- **THEN** it is placed at 1008 x 604 inside a sector (sizes round down, so it never exceeds the space), and its original is still 1500 x 900

### Requirement: Tray
When a new tile fits in no sector, it SHALL be placed on the canvas outside all sectors, next to the sector grid, without overlapping other tiles. Canvas outside sectors is the tray. Tiles in the tray SHALL never appear in an export, and the status SHALL say that the tile went to the tray.

#### Scenario: All sectors full
- **WHEN** every sector is full and a tile arrives from ShareX
- **THEN** the tile lies on the canvas beside the grid, no sector content changes, and the status reports that the tile went to the tray

### Requirement: Select and move
In ATLAS mode, clicking a tile SHALL select it. A click SHALL hit a tile only inside its rotated outline. Shift+click SHALL add a tile to the selection or remove it. Dragging a selected tile SHALL move the whole selection freely, in or out of sectors and across sector borders. Clicking empty canvas SHALL clear the selection, and Esc SHALL clear it too.

#### Scenario: Drag a tile out to the tray
- **WHEN** the user drags a tile from sector 1001 onto empty canvas
- **THEN** the tile stays where it is dropped and is no longer part of sector 1001's export

#### Scenario: Click beside a rotated tile
- **WHEN** a tile is turned 45 degrees and the user clicks inside its bounding box but outside the tile itself
- **THEN** the tile is not selected and the click acts on what lies under that point

### Requirement: Scale
A single selected tile SHALL show handles at the corners of its rotated frame. Dragging a handle SHALL scale the tile uniformly along its own axes, keeping its aspect ratio, about the opposite corner, or about the pivot point while Alt is held. While scaling, the status SHALL show the tile's size before rotation. A tile SHALL not be scaled below 1 x 1 pixel.

#### Scenario: Scale with a corner handle
- **WHEN** the user drags the bottom-right handle of a 512 x 256 tile until it is 1024 pixels wide
- **THEN** the tile is 1024 x 512, its top-left corner has not moved, and the status shows 1024 x 512

#### Scenario: Scale a rotated tile
- **WHEN** a 512 x 256 tile is turned 30 degrees and the user drags one of its handles outward
- **THEN** the tile grows along its own turned axes, stays turned 30 degrees, the opposite corner stays in place, and the status shows its size before rotation

#### Scenario: Scale about the pivot
- **WHEN** the pivot is at the centre and the user Alt+drags a handle of a 512 x 256 tile until it is 1024 pixels wide
- **THEN** the tile is 1024 x 512 with its centre unchanged

### Requirement: Rotate by 90 degrees
R SHALL rotate the selected tiles by 90 degrees clockwise, and Shift+R counter-clockwise, each about its own pivot point, adding to any angle the tile already has. Rotation by multiples of 90 degrees SHALL be lossless: rotating four times returns the exact original placement and pixels.

#### Scenario: Rotate a tile
- **WHEN** the user presses R with a 400 x 200 tile selected
- **THEN** the tile is shown as 200 x 400 with its content turned clockwise, centred where it was

#### Scenario: Quarter turn on a free angle
- **WHEN** the user presses R on a tile turned 17 degrees
- **THEN** the tile is turned 107 degrees

#### Scenario: Four turns about a moved pivot
- **WHEN** a tile's pivot is on its bottom-right corner and the user presses R four times
- **THEN** the tile is back at its exact original placement

### Requirement: Delete
Delete SHALL remove the selected tiles and their originals from the workspace. The status SHALL report how many tiles were removed. The change is written to the working copy at once. The `.ripper` file still holds them until the next save.

#### Scenario: Delete two tiles
- **WHEN** two tiles are selected and the user presses Delete
- **THEN** both disappear from the canvas and the status reports 2 tiles removed

### Requirement: Tiles across sector borders
A tile MAY lie across the border between sectors or partly outside the grid. In every export, each sector SHALL contain exactly the part of the tile that lies within that sector.

#### Scenario: Tile on a border
- **WHEN** a tile lies half in sector 1001 and half in sector 1002
- **THEN** exporting 1001 shows its left half and exporting 1002 shows its right half, and the halves join without a gap or overlap

### Requirement: Free rotation
A single selected tile SHALL have a rotate zone just outside each corner, about 16 screen pixels deep. Dragging in it SHALL turn the tile about its pivot point by the angle swept around the pivot. Shift SHALL snap the tile's angle to multiples of 15 degrees. Over the zone the cursor SHALL show a rotate icon, and while rotating the status SHALL show the tile's angle.

#### Scenario: Turn a tile by dragging
- **WHEN** the user drags from just outside the top-right corner of a tile so that the pointer sweeps 30 degrees clockwise around the pivot
- **THEN** the tile is turned 30 degrees clockwise about its pivot and the status shows its angle

#### Scenario: Snap with Shift
- **WHEN** the user holds Shift while rotating a tile to about 37 degrees
- **THEN** the tile's angle is 30 degrees

#### Scenario: Handle and zone stay apart
- **WHEN** the pointer is over a corner handle of the selected tile
- **THEN** dragging scales the tile, and only dragging from outside the frame rotates it

### Requirement: Pivot point
Every tile SHALL have a pivot point, at its centre by default, stored in the workspace and moving with the tile. It SHALL be shown on a single selected tile. Alt+drag SHALL move it, also outside the tile, snapping to the corners, edge midpoints and centre when near. Alt+double-click SHALL reset it to the centre. A moved pivot SHALL sit on whole pixels.

#### Scenario: Rotate about a corner
- **WHEN** the user Alt+drags the pivot of a 400 x 200 tile onto its top-left corner and presses R
- **THEN** the tile turns 90 degrees clockwise with its former top-left corner staying in place

#### Scenario: Pivot is kept
- **WHEN** the user moves a tile's pivot, moves the tile, saves `cafe.ripper` and opens it again
- **THEN** the pivot is at the same place on the tile

#### Scenario: Reset the pivot
- **WHEN** the user Alt+double-clicks the selected tile's pivot
- **THEN** the pivot returns to the tile's centre

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

### Requirement: Marquee selection
In ATLAS mode, dragging from empty canvas, inside a sector or outside, SHALL draw a selection frame once the pointer has moved 3 screen pixels. The frame SHALL select every tile whose rotated outline it touches. Without Shift the frame SHALL replace the tile selection; with Shift it SHALL add to it. Tiles SHALL show as selected while the frame moves. The frame SHALL NOT change the selected sectors or the active sector.

#### Scenario: Frame across two tiles
- **GIVEN** three tiles in sector 1001, none selected
- **WHEN** the user drags from empty space in the sector across the first two tiles and releases
- **THEN** exactly the first two tiles are selected, and the sector selection and active sector are as before the drag

#### Scenario: Touching is enough
- **WHEN** the frame covers only a corner of a tile
- **THEN** the tile is selected

#### Scenario: Rotated outline, not the bounding box
- **GIVEN** a tile turned 45 degrees
- **WHEN** the frame covers a corner of the tile's bounding box but not the tile itself
- **THEN** the tile is not selected

#### Scenario: Frame replaces the selection
- **GIVEN** tile A is selected
- **WHEN** the user draws a frame without Shift that touches only tile B
- **THEN** only tile B is selected

#### Scenario: Shift adds to the selection
- **GIVEN** tile A is selected
- **WHEN** the user draws a frame with Shift that touches tile B
- **THEN** tiles A and B are selected

#### Scenario: Live selection
- **WHEN** the frame reaches a tile and then moves back off it before release
- **THEN** the tile shows as selected while it is touched and not after the frame leaves it

#### Scenario: Frame from the tray
- **WHEN** the user drags from empty canvas outside all sectors across a tile in the tray and a tile in a sector
- **THEN** both tiles are selected

### Requirement: Click on empty canvas is unchanged
A press and release on empty canvas that moves less than 3 screen pixels SHALL act as a click, as before marquee selection: inside a sector it selects that sector, with Shift it toggles that sector, and outside all sectors it clears the selection. Esc during a frame drag SHALL cancel the frame and restore the selection it started from.

#### Scenario: Click selects a sector
- **WHEN** the user clicks empty space in sector 1002 without moving the pointer
- **THEN** sector 1002 is selected and becomes active, and no frame is drawn

#### Scenario: Small jitter is still a click
- **WHEN** the pointer moves 2 screen pixels between press and release on empty canvas outside all sectors
- **THEN** the selection is cleared and no frame is drawn

#### Scenario: Esc cancels the frame
- **GIVEN** tile A is selected
- **WHEN** the user draws a frame across tile B and presses Esc before release
- **THEN** the frame disappears and only tile A is selected

### Requirement: Marquee look
The selection frame SHALL be drawn from the press point to the pointer as marching ants: a 1 CSS-pixel line of alternating black and white 4-pixel dashes that keep moving while the frame is up, even when the pointer rests, with no fill. It SHALL disappear on release or Esc. A marquee SHALL add no undo step.

#### Scenario: Ants march while the pointer rests
- **WHEN** the user stops moving the pointer while drawing a frame
- **THEN** the dashes keep moving along the frame

#### Scenario: Frame disappears on release
- **WHEN** the user releases the pointer after drawing a frame
- **THEN** no frame is drawn and the touched tiles stay selected

#### Scenario: No undo step
- **GIVEN** a tile was moved and then a frame selected two other tiles
- **WHEN** the user presses Ctrl+Z
- **THEN** the move is undone
