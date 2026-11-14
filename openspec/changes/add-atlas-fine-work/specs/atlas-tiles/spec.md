## ADDED Requirements

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

## MODIFIED Requirements

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

### Requirement: Select and move
In ATLAS mode, clicking a tile SHALL select it. A click SHALL hit a tile only inside its rotated outline. Shift+click SHALL add a tile to the selection or remove it. Dragging a selected tile SHALL move the whole selection freely, in or out of sectors and across sector borders. Clicking empty canvas SHALL clear the selection, and Esc SHALL clear it too.

#### Scenario: Drag a tile out to the tray
- **WHEN** the user drags a tile from sector 1001 onto empty canvas
- **THEN** the tile stays where it is dropped and is no longer part of sector 1001's export

#### Scenario: Click beside a rotated tile
- **WHEN** a tile is turned 45 degrees and the user clicks inside its bounding box but outside the tile itself
- **THEN** the tile is not selected and the click acts on what lies under that point
