## ADDED Requirements

### Requirement: Edge fill
The Export panel SHALL offer Fill TRANSPARENT or EXTEND, and Edge in pixels, 16 by default, from 1 to the sector size. With EXTEND, each empty pixel within Edge pixels of a tile SHALL take the colour of the nearest tile pixel and be opaque. Pixels farther away SHALL stay transparent. Fill and Edge SHALL be stored in the workspace.

#### Scenario: Extend by 16 pixels
- **WHEN** a sector holds one opaque red tile and is exported with Fill EXTEND and Edge 16
- **THEN** the 16 pixels around the tile are opaque red and the pixels beyond them are transparent

#### Scenario: Two tiles meet halfway
- **WHEN** a red and a blue tile are 8 pixels apart and the sector is exported with EXTEND and Edge 16
- **THEN** the gap is filled red on the red tile's side and blue on the blue tile's side, meeting in the middle

#### Scenario: Soft edge becomes solid
- **WHEN** a tile turned 30 degrees is exported with EXTEND
- **THEN** its antialiased edge lies over the extended colour and no transparent or dark fringe remains

#### Scenario: Setting is kept
- **WHEN** the user picks EXTEND with Edge 32, saves `cafe.ripper` and opens it later
- **THEN** the Export panel shows EXTEND and 32

### Requirement: Edge fill per sector
Edge fill SHALL be computed for each sector on its own, from the parts of tiles that lie inside that sector. Colour SHALL never spread across a sector border. SECTOR, GROUP FILE and GROUP SET SHALL produce the same pixels for a sector.

#### Scenario: Tile near a border
- **WHEN** a tile in sector 1001 ends 4 pixels before the border with sector 1002 and both are exported with EXTEND and Edge 16
- **THEN** sector 1001 is filled up to its border and sector 1002 gets none of that tile's colour

#### Scenario: Same pixels in every scope
- **WHEN** sectors 1001 and 1002 are exported with EXTEND once as GROUP FILE and once as GROUP SET
- **THEN** each half of the sheet equals the corresponding file of the set pixel for pixel

## MODIFIED Requirements

### Requirement: Tile rendering in exports
Every tile SHALL be rendered into the export from its original. Rotations by multiples of 90 degrees are exact. Free rotation and any scaling SHALL be filtered so that tiles stay free of jagged edges and moire, at least as well as the RIP export, and the edges of a freely rotated tile SHALL be antialiased. With Fill TRANSPARENT, areas without a tile SHALL be fully transparent. Tiles in the tray SHALL never appear.

#### Scenario: Shrunk fine pattern
- **WHEN** a tile with a fine stripe pattern is scaled to a quarter of its original width and exported
- **THEN** the stripes in the export average out smoothly, with no moire bands

#### Scenario: Empty sector
- **WHEN** a sector with no tiles is exported
- **THEN** the PNG is fully transparent

#### Scenario: Freely rotated tile
- **WHEN** a tile turned 30 degrees at scale 1 is exported with Fill TRANSPARENT
- **THEN** its edges show a smooth partial-alpha transition instead of a staircase, and the area around it is transparent
