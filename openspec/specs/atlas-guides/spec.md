# atlas-guides Specification

## Purpose
Defines visual aids on atlas sectors that help place tiles but never reach an export: UV-layout guide images per sector and the graph-paper grid drawn in every sector.

## Requirements

### Requirement: Guides panel
ATLAS mode SHALL have a Guides panel between the Sector and Export panels. It SHALL offer LOAD..., REMOVE, a Show checkbox and an Opacity control, and name the sector REMOVE acts on. REMOVE SHALL be disabled when the selected sector has no guide.

#### Scenario: Panel follows the selection
- **WHEN** the user selects sector 1002, which has a guide
- **THEN** the Guides panel names 1002 and REMOVE is enabled

### Requirement: Load a single guide
LOAD... SHALL accept one or more PNG files. A file whose name does not end in `.<udim>.png` SHALL become the guide of the selected sector, or of the active sector when none is selected. Choosing more than one such file SHALL be refused with an explanation. A new guide SHALL replace the sector's existing guide.

#### Scenario: Guide on the selected sector
- **WHEN** sector 1001 is selected and the user loads `body_uv.png`
- **THEN** sector 1001 shows that image as its guide and the status reports one guide loaded

#### Scenario: Replace a guide
- **WHEN** sector 1001 already has a guide and the user loads another PNG onto it
- **THEN** only the new guide is shown on 1001

### Requirement: Load a numbered guide set
Files named `<name>.<udim>.png` with a number from 1001 to 1999 SHALL go to sectors by number. 1001 is the bottom-left cell of the whole workspace grid: column = (number - 1001) mod 10, row = (number - 1001) div 10. A missing sector SHALL be created. The status SHALL report guides loaded and sectors created.

#### Scenario: Set into a one-sector workspace
- **WHEN** a new workspace has one sector and the user loads `body.1001.png` to `body.1004.png`
- **THEN** the workspace has four sectors in one row, each with its guide, and the status reports 4 guides loaded and 3 sectors created

#### Scenario: Numbering ignores the selection
- **WHEN** the grid has sectors 1001 and 1002, sector 1002 is selected and the user loads `body.1001.png`
- **THEN** the guide goes to the bottom-left sector 1001, not to 1002

#### Scenario: Second row
- **WHEN** the user loads `body.1011.png` into a workspace whose grid is one row
- **THEN** a sector is created directly above the bottom-left sector and gets the guide

### Requirement: Remove a guide
REMOVE SHALL delete the guide of the selected sector. Removing a guide SHALL NOT change tiles or sectors.

#### Scenario: Remove
- **WHEN** sector 1002 is selected and the user clicks REMOVE
- **THEN** sector 1002 shows no guide and its tiles are unchanged

### Requirement: Guide belongs to its sector
A guide SHALL stay with its sector. Removing a sector SHALL remove its guide too, and the rule that only a sector without tiles can be removed SHALL stay unchanged.

#### Scenario: Remove a sector with a guide
- **WHEN** sector 1002 has a guide and no tiles and the user removes it
- **THEN** the sector and its guide are gone, and undoing the removal brings back both

### Requirement: Guide fitting
A guide SHALL be drawn stretched to exactly cover its sector square, whatever its pixel size. A guide that is not square SHALL still be stretched, and the status SHALL warn that it is not square. Guide colours SHALL be shown as they are in the file.

#### Scenario: Large guide on a small sector
- **WHEN** a 4096 x 4096 guide is loaded onto a 1024 sector
- **THEN** it covers the sector edge to edge

#### Scenario: Non-square guide
- **WHEN** a 2048 x 1024 PNG is loaded as a guide
- **THEN** it is stretched over the whole sector and the status warns that the guide is not square

### Requirement: Guide display under tiles
Guides SHALL be drawn above the graph-paper grid and below all tiles. Show and Opacity SHALL apply to all guides of the workspace and be stored in it. A new workspace SHALL have Show on and Opacity 50 %. With Show off, guides SHALL not be drawn except during the preview.

#### Scenario: Opacity is kept
- **WHEN** the user sets Opacity to 30 %, saves `cafe.ripper` and opens it later
- **THEN** the guides are drawn at 30 % opacity

#### Scenario: Tiles cover the guide
- **WHEN** a tile lies over part of a sector's guide
- **THEN** the guide is not visible through the opaque parts of the tile

### Requirement: Guide preview above tiles
While the user holds G in ATLAS mode, every guide SHALL be drawn above the tiles in magenta `#FF00FF` at full opacity, using the guide's own alpha as the mask. Releasing G, or the window losing focus, SHALL return to the normal display. The preview SHALL work whether Show is on or off.

#### Scenario: Check the UV islands
- **WHEN** the user holds G over a sector with tiles and a white-line guide
- **THEN** the guide lines appear in magenta over the tiles, and after G is released they are under the tiles again at the set opacity

### Requirement: Guides are never exported
No export scope SHALL include guides or the graph-paper grid. Guides SHALL be kept in the workspace and in its `.ripper` file.

#### Scenario: Export with guides shown
- **WHEN** a sector with a guide and no tiles is exported with Fill TRANSPARENT
- **THEN** the PNG is fully transparent

### Requirement: Graph-paper grid
Every sector SHALL always show a graph-paper grid under guides and tiles: major lines every 1/8 of the sector size and minor lines every 1/64. Minor lines SHALL be hidden while they would be closer than about 6 screen pixels. The grid uses the design system's line colours.

#### Scenario: Grid on a 2048 sector
- **WHEN** a 2048 sector is shown at 100 % zoom
- **THEN** major lines appear every 256 pixels and minor lines every 32 pixels

#### Scenario: Zoomed out
- **WHEN** the view is zoomed out until minor lines would be 3 screen pixels apart
- **THEN** only the major lines are drawn
