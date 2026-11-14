# atlas-tiles Specification

## Purpose

Defines tiles, the textures placed on an atlas workspace: where a new tile lands, how it is kept at full quality, and how the user moves, scales, rotates and removes it.

## Requirements

### Requirement: Tile keeps its original
Every tile SHALL keep the full-size image it was created from, in the workspace. The tile's size on the canvas is a scale of that original. Every resize and every export SHALL be rendered from the original, never from an earlier rendered copy.

#### Scenario: Shrink and enlarge again
- **WHEN** a 1600 x 900 tile is scaled down to 200 x 113 and later scaled back to 1600 x 900
- **THEN** the exported tile matches the original pixel for pixel

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
In ATLAS mode, clicking a tile SHALL select it. Shift+click SHALL add a tile to the selection or remove it. Dragging a selected tile SHALL move the whole selection freely, in or out of sectors and across sector borders. Clicking empty canvas SHALL clear the selection, and Esc SHALL clear it too.

#### Scenario: Drag a tile out to the tray
- **WHEN** the user drags a tile from sector 1001 onto empty canvas
- **THEN** the tile stays where it is dropped and is no longer part of sector 1001's export

### Requirement: Scale
A single selected tile SHALL show corner handles. Dragging a handle SHALL scale the tile uniformly, keeping its aspect ratio, about the opposite corner. While scaling, the status SHALL show the tile's current pixel size. A tile SHALL not be scaled below 1 x 1 pixel.

#### Scenario: Scale with a corner handle
- **WHEN** the user drags the bottom-right handle of a 512 x 256 tile until it is 1024 pixels wide
- **THEN** the tile is 1024 x 512, its top-left corner has not moved, and the status shows 1024 x 512

### Requirement: Rotate by 90 degrees
R SHALL rotate the selected tiles by 90 degrees clockwise, and Shift+R counter-clockwise, each about its own centre. Rotation by multiples of 90 degrees SHALL be lossless: rotating four times returns the exact original placement and pixels.

#### Scenario: Rotate a tile
- **WHEN** the user presses R with a 400 x 200 tile selected
- **THEN** the tile is shown as 200 x 400 with its content turned clockwise, centred where it was

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
