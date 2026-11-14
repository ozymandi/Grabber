## ADDED Requirements

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
