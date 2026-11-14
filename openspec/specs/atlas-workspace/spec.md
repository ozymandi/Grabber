# atlas-workspace Specification

## Purpose

Defines the atlas document: an infinite canvas of equal power-of-two sectors arranged as a UDIM grid, how it is created, opened and saved, which workspace receives new tiles, and how it shares the window with the RIP editor.

## Requirements

### Requirement: RIP and ATLAS modes
The window SHALL have two modes, RIP and ATLAS. A RIP | ATLAS segmented switch at the bottom-left of the canvas area selects the mode in both modes. Each mode shows its own canvas and panels, and the status row and help stay shared. A ShareX launch SHALL always open in RIP mode.

#### Scenario: Switch to ATLAS
- **WHEN** the user clicks ATLAS while editing quads
- **THEN** the canvas shows the current workspace and the side panel shows the atlas panels, and switching back to RIP shows the same image and quads as before

#### Scenario: ShareX opens RIP
- **WHEN** ShareX starts the app while the previous session ended in ATLAS mode
- **THEN** the app opens in RIP mode with the captured image

### Requirement: Workspace document
A workspace SHALL be saved as a single `.ripper` file. The file holds the sectors, every tile's placement, rotation and pivot, every tile's picture history with the full-size picture of each state, the sector histories, the sector guides, and the workspace's guide Show and Opacity and export Fill and Edge. ATLAS mode SHALL offer NEW, OPEN, SAVE and SAVE AS. SAVE on an untitled workspace SHALL act as SAVE AS.

#### Scenario: Save and reopen
- **WHEN** the user saves a workspace with six tiles as `cafe.ripper`, starts NEW, and then opens `cafe.ripper`
- **THEN** the six tiles reappear in the same sectors, positions, sizes and rotations

#### Scenario: Guides travel with the file
- **WHEN** a workspace with guides on sectors 1001 and 1002 is saved as `cafe.ripper` and opened on another machine
- **THEN** both guides appear on the same sectors

#### Scenario: History travels with the file
- **WHEN** a tile with three picture states is saved in `cafe.ripper` and the file is opened again
- **THEN** the tile shows the same current state and its History lists the same three states

### Requirement: Workspace name
An unsaved workspace SHALL be nameable in the Atlas slot of the RIP Output panel and in the Workspace panel of ATLAS mode; Enter or leaving the field commits, Esc reverts. The name SHALL be the default file name on SAVE and the default export name. Names that are empty, longer than 100 characters, contain `< > : " / \ | ? *` or end with a dot SHALL be refused with an explanation. A saved workspace SHALL be named after its file: both fields are read-only, and renaming is SAVE AS.

#### Scenario: Name before the first save
- **WHEN** the user types `cafe facades` into the Atlas field of an untitled workspace and presses Enter
- **THEN** the workspace is named `cafe facades`, and SAVE suggests `cafe facades.ripper`

#### Scenario: Saved workspace
- **WHEN** the workspace has been saved as `cafe.ripper`
- **THEN** both name fields show `cafe`, cannot be edited, and their tooltip says to use SAVE AS

### Requirement: Current workspace
The current workspace SHALL be the one created or opened last. It SHALL be remembered across launches, including ShareX launches. New tiles SHALL always go to the current workspace. NEW SHALL make an empty untitled workspace current.

#### Scenario: Captures go to the opened workspace
- **WHEN** the user opens `cafe.ripper`, closes the app and later sends a quad from ShareX with the Atlas output on
- **THEN** the tile is added to `cafe.ripper`'s workspace

### Requirement: Edits are kept without saving
Every change to a workspace SHALL be written to its working copy at once. Changes include adding, moving, scaling, rotating and deleting tiles, moving pivots, adding or removing sectors, loading or removing guides, changing guide Show or Opacity and changing Fill or Edge. Changes not yet saved to the `.ripper` file SHALL survive closing the window or the app. The workspace title SHALL mark unsaved changes.

#### Scenario: Window closed before saving
- **WHEN** the user moves three tiles in `cafe.ripper` and closes the app without saving
- **THEN** on the next launch `cafe.ripper` is current, shows the three moved tiles, and is marked as having unsaved changes

#### Scenario: ShareX tile then save
- **WHEN** a tile arrives from ShareX into `cafe.ripper` and the user later presses Ctrl+S in ATLAS mode
- **THEN** the `.ripper` file contains that tile and the unsaved mark is cleared

#### Scenario: Guide loaded before closing
- **WHEN** the user loads a guide onto sector 1001 and closes the app without saving
- **THEN** on the next launch sector 1001 shows the guide and the workspace is marked as having unsaved changes

### Requirement: Sector size
Each workspace SHALL have one sector size, chosen from 512, 1024, 2048, 4096 and 8192 pixels. Every sector is square at that size. A new workspace SHALL use 2048. The size SHALL be changeable only while the workspace has no tiles.

#### Scenario: Size locked after the first tile
- **WHEN** a workspace with sector size 1024 contains one tile
- **THEN** the sector size control is disabled and explains that the workspace must be empty to change it

### Requirement: Sector grid
A new workspace SHALL start with one sector. When one sector is selected, "+" buttons SHALL appear on each of its sides that has no neighbour. Clicking one SHALL add a sector flush against that side, on the same grid. A sector with no tile overlapping it SHALL be removable. Sectors SHALL never overlap or leave gaps between neighbours.

#### Scenario: Add a sector to the right
- **WHEN** the user selects the only sector and clicks its right "+"
- **THEN** a second sector of the same size appears touching its right edge, and the first sector no longer shows a right "+"

#### Scenario: Remove a sector
- **WHEN** a sector has no tile on it and the user removes it
- **THEN** the sector disappears and its neighbours show "+" on the freed side

#### Scenario: Sector with tiles
- **WHEN** a tile overlaps a sector
- **THEN** that sector cannot be removed and the status explains why

### Requirement: Active sector
The sector selected last in a workspace SHALL be its active sector, and the choice SHALL be stored in the workspace. New tiles SHALL be placed into the active sector first. If no sector has been selected, the first sector with free space in UDIM order SHALL be used.

#### Scenario: Active sector remembered
- **WHEN** the user selects sector 1002, closes the app and a tile arrives from ShareX
- **THEN** the tile is placed into sector 1002 if it fits there

### Requirement: Workspace padding
Each workspace SHALL have a padding value in pixels, 8 by default. Automatic placement keeps at least this distance between tiles and from sector edges. Changing the padding SHALL NOT move existing tiles.

#### Scenario: Padding applies to new tiles only
- **WHEN** the user changes the padding from 8 to 16
- **THEN** existing tiles stay where they are and the next automatically placed tile keeps 16 pixels of clearance

### Requirement: Concurrent writers stay consistent
A tile may be added from a ShareX launch while another ripper window has the same workspace open in ATLAS mode. Neither change SHALL be lost, and the open window SHALL show the new tile within two seconds. Changes made in either window SHALL never overwrite the other's.

#### Scenario: Capture arrives while the atlas is open
- **WHEN** the atlas is open in one window and the user sends a quad from a ShareX capture with the Atlas output on
- **THEN** the tile appears in the open window within two seconds, and a tile the user moved there a moment earlier keeps its new position

### Requirement: Document format compatibility
Workspaces and `.ripper` files written by ripper 2.0.0 SHALL open with every tile in the same place, size and quarter-turn rotation, with pivots at tile centres, no guides, guides shown at 50 % and Fill TRANSPARENT. A file written by a newer format than the app understands SHALL be refused with an explanation, and nothing SHALL be changed.

#### Scenario: Open a 2.0.0 file
- **WHEN** the user opens `cafe.ripper` saved by 2.0.0 with six tiles, two of them rotated by 90 degrees
- **THEN** the six tiles appear exactly as in 2.0.0 and exporting a sector gives the same pixels as 2.0.0 did

#### Scenario: File from a newer version
- **WHEN** the user opens a `.ripper` file written by a newer ripper format
- **THEN** the file is not opened, the current workspace stays as it was, and the status says the file needs a newer ripper
