# atlas-export Specification

## Purpose

Defines how sectors of an atlas workspace become image files: one sector, a group of sectors as one sheet, or a group as a numbered UDIM set, rendered from the tiles' originals without overwriting existing files.

## Requirements

### Requirement: Sector selection for export
In ATLAS mode, clicking a sector's empty area SHALL select it. Shift+click SHALL add sectors to or remove them from the selection. Export SHALL act on the selected sectors. If none are selected, it SHALL act on the active sector.

#### Scenario: Select three sectors
- **WHEN** the user clicks sector 1001 and Shift+clicks 1002 and 1011
- **THEN** the three sectors are highlighted and the Export panel names 3 sectors

### Requirement: Export one sector
SECTOR export SHALL write one PNG, the size of a sector, for each selected sector. It SHALL contain the parts of all tiles that lie within that sector.

#### Scenario: Export a single 2048 sector
- **WHEN** one 2048 sector is selected and the user exports with scope SECTOR
- **THEN** one 2048 x 2048 PNG is written with that sector's tiles in place

### Requirement: Export a group as one file
GROUP FILE export SHALL write a single PNG covering the bounding rectangle of the selected sectors. Each sector keeps its grid position. Cells of the rectangle that are not selected SHALL be transparent. A sheet larger than 16384 pixels on either side SHALL be refused with an explanation.

#### Scenario: Two by two sheet
- **WHEN** four 1024 sectors forming a 2 x 2 block are selected and exported with scope GROUP FILE
- **THEN** one 2048 x 2048 PNG is written, and a tile on the border between two sectors appears whole

#### Scenario: L-shaped selection
- **WHEN** sectors 1001, 1002 and 1011 are selected
- **THEN** the sheet is two sectors wide and two high, and the cell of 1012 is fully transparent

#### Scenario: Sheet too large
- **WHEN** a 3 x 1 block of 8192 sectors is exported with scope GROUP FILE
- **THEN** no file is written and the status explains the 16384 pixel limit and suggests GROUP SET

### Requirement: Export a group as a UDIM set
GROUP SET export SHALL write one PNG per selected sector, named `<name>.<udim>.png`. The UDIM number is 1001 plus the column, plus 10 times the row. Columns and rows count from the selection's bottom-left sector. A selection more than 10 sectors wide SHALL be refused with an explanation.

#### Scenario: Set numbered from the bottom-left
- **WHEN** a sector is added to the left of the first sector and both are exported with scope GROUP SET and name `cafe`
- **THEN** the files are `cafe.1001.png` for the left sector and `cafe.1002.png` for the right one

#### Scenario: Too wide for UDIM
- **WHEN** eleven sectors in one row are exported with scope GROUP SET
- **THEN** no file is written and the status explains that UDIM sets are at most 10 sectors wide

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

### Requirement: Export destination and naming
Export SHALL ask for a folder and a base name; the base name defaults to the workspace name. Existing files SHALL never be overwritten. If any target name exists, `_2`, `_3` and so on SHALL be appended to the base name. For a UDIM set the same suffix SHALL apply to every file of the set, so the set keeps one name.

#### Scenario: Name collision in a set
- **WHEN** `cafe.1001.png` already exists in the folder and a two-sector set named `cafe` is exported there
- **THEN** the set is written as `cafe_2.1001.png` and `cafe_2.1002.png`, and the existing file is unchanged

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
