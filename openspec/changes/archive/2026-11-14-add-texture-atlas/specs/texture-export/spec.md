# Spec Delta

## MODIFIED Requirements

### Requirement: Send active quad
SEND ACTIVE and Enter SHALL export the active quad as PNG. The PNG goes to the clipboard and to every enabled output folder, and in ShareX mode also to the output file. When the Atlas output is on, the quad SHALL also be added as a tile to the current atlas workspace. In ShareX mode the app SHALL then exit with code 0. In standalone mode it SHALL stay open.

#### Scenario: Send in ShareX mode
- **WHEN** in ShareX mode the user presses Enter with a valid active quad
- **THEN** the PNG is written to the output file and each enabled folder, the image is on the clipboard and the app exits

#### Scenario: Send in standalone mode
- **WHEN** in standalone mode the user presses Enter
- **THEN** the PNG is on the clipboard and in each enabled folder, and the app stays open with a success status

#### Scenario: Nothing to send
- **WHEN** Enter is pressed with no quads or an invalid active quad
- **THEN** nothing is written, the app stays open and the status row explains why

#### Scenario: Send adds a tile to the atlas
- **WHEN** the Atlas output is on, the current workspace is `cafe.ripper` and the user presses Enter in ShareX mode
- **THEN** the quad is also added as a tile to `cafe.ripper`'s workspace before the app exits

### Requirement: Save all quads
SAVE ALL and Ctrl+S SHALL export every valid quad as a separate PNG into every enabled output folder. When the Atlas output is on, every valid quad SHALL also be added as a tile to the current atlas workspace, in Quads panel order. The app SHALL stay open, and the clipboard and the ShareX output file SHALL be left untouched.

#### Scenario: Save three quads
- **WHEN** the image has three valid quads, both folders are enabled and the user presses Ctrl+S
- **THEN** three PNGs are written into each folder and the status reports 3 saved

#### Scenario: Some quads invalid
- **WHEN** one of three quads is degenerate
- **THEN** the two valid quads are saved and the status names the skipped one

#### Scenario: Save all into the atlas
- **WHEN** the Atlas output is on, no folder is enabled and the image has three valid quads
- **THEN** three tiles are added to the current workspace and the status reports them

## ADDED Requirements

### Requirement: Atlas output
The Output panel SHALL offer a third output, Atlas, next to Local and Google Drive. It SHALL have an on/off toggle and show the name of the current workspace, or "Untitled" for an unsaved one. The toggle SHALL persist with the other settings, including for ShareX launches.

#### Scenario: Atlas toggle survives restart
- **WHEN** the user turns the Atlas output on, closes the app and ShareX starts it again
- **THEN** the Atlas output is on and names the same current workspace

#### Scenario: Atlas output off
- **WHEN** the Atlas output is off and the user sends a quad
- **THEN** no tile is added to any workspace

### Requirement: Atlas failure does not block other outputs
If a tile cannot be added to the workspace, the app SHALL still complete the clipboard, the ShareX output and the folders. A failure includes a disk error and a workspace that stays locked by another ripper window. The failure SHALL be reported with a warning status, and in ShareX mode it SHALL keep the app open, as for a folder failure.

#### Scenario: Workspace cannot be written
- **WHEN** in ShareX mode the Atlas output is on and the workspace's working folder cannot be written
- **THEN** the output file, the clipboard and the enabled folders are written, a warning names the atlas, and the app stays open
