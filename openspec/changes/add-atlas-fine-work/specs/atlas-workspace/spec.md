## ADDED Requirements

### Requirement: Document format compatibility
Workspaces and `.ripper` files written by ripper 2.0.0 SHALL open with every tile in the same place, size and quarter-turn rotation, with pivots at tile centres, no guides, guides shown at 50 % and Fill TRANSPARENT. A file written by a newer format than the app understands SHALL be refused with an explanation, and nothing SHALL be changed.

#### Scenario: Open a 2.0.0 file
- **WHEN** the user opens `cafe.ripper` saved by 2.0.0 with six tiles, two of them rotated by 90 degrees
- **THEN** the six tiles appear exactly as in 2.0.0 and exporting a sector gives the same pixels as 2.0.0 did

#### Scenario: File from a newer version
- **WHEN** the user opens a `.ripper` file written by a newer ripper format
- **THEN** the file is not opened, the current workspace stays as it was, and the status says the file needs a newer ripper

## MODIFIED Requirements

### Requirement: Workspace document
A workspace SHALL be saved as a single `.ripper` file. The file holds the sectors, every tile's placement, rotation and pivot, every tile's full-size original image, the sector guides, and the workspace's guide Show and Opacity and export Fill and Edge. ATLAS mode SHALL offer NEW, OPEN, SAVE and SAVE AS. SAVE on an untitled workspace SHALL act as SAVE AS.

#### Scenario: Save and reopen
- **WHEN** the user saves a workspace with six tiles as `cafe.ripper`, starts NEW, and then opens `cafe.ripper`
- **THEN** the six tiles reappear in the same sectors, positions, sizes and rotations

#### Scenario: Guides travel with the file
- **WHEN** a workspace with guides on sectors 1001 and 1002 is saved as `cafe.ripper` and opened on another machine
- **THEN** both guides appear on the same sectors

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
