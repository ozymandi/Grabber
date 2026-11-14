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
Every tile SHALL be rendered into the export from its original. Rotations by 90 degrees are exact. Any scaling SHALL be filtered so that a shrunk tile stays free of jagged edges and moire, at least as well as the RIP export. Areas without a tile SHALL be fully transparent. Tiles in the tray SHALL never appear.

#### Scenario: Shrunk fine pattern
- **WHEN** a tile with a fine stripe pattern is scaled to a quarter of its original width and exported
- **THEN** the stripes in the export average out smoothly, with no moire bands

#### Scenario: Empty sector
- **WHEN** a sector with no tiles is exported
- **THEN** the PNG is fully transparent

### Requirement: Export destination and naming
Export SHALL ask for a folder and a base name; the base name defaults to the workspace name. Existing files SHALL never be overwritten. If any target name exists, `_2`, `_3` and so on SHALL be appended to the base name. For a UDIM set the same suffix SHALL apply to every file of the set, so the set keeps one name.

#### Scenario: Name collision in a set
- **WHEN** `cafe.1001.png` already exists in the folder and a two-sector set named `cafe` is exported there
- **THEN** the set is written as `cafe_2.1001.png` and `cafe_2.1002.png`, and the existing file is unchanged
