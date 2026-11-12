# texture-export Specification

## Purpose

Sets where flattened textures go and how the app fits into ShareX's capture flow. Covers the clipboard, the ShareX output file and two user-configured output folders that persist between runs.

## Requirements

### Requirement: ShareX launch contract
When the app is started with a second argument, it SHALL run in ShareX mode and treat that argument as the output file path. ShareX passes it as `$output`. With only one argument, or none, the app SHALL run in standalone mode.

#### Scenario: ShareX mode
- **WHEN** ShareX runs `ripper.exe "C:\shots\a.png" "C:\shots\a_out.png"`
- **THEN** the app opens `a.png` in ShareX mode with `a_out.png` as its output file

#### Scenario: Standalone mode
- **WHEN** the app is started with only an image path
- **THEN** it runs in standalone mode and stays open after exporting

### Requirement: Send active quad
SEND ACTIVE and Enter SHALL export the active quad as PNG. The PNG goes to the clipboard and to every enabled output folder, and in ShareX mode also to the output file. In ShareX mode the app SHALL then exit with code 0. In standalone mode it SHALL stay open.

#### Scenario: Send in ShareX mode
- **WHEN** in ShareX mode the user presses Enter with a valid active quad
- **THEN** the PNG is written to the output file and each enabled folder, the image is on the clipboard and the app exits

#### Scenario: Send in standalone mode
- **WHEN** in standalone mode the user presses Enter
- **THEN** the PNG is on the clipboard and in each enabled folder, and the app stays open with a success status

#### Scenario: Nothing to send
- **WHEN** Enter is pressed with no quads or an invalid active quad
- **THEN** nothing is written, the app stays open and the status row explains why

### Requirement: Save all quads
SAVE ALL and Ctrl+S SHALL export every valid quad as a separate PNG into every enabled output folder. The app SHALL stay open, and the clipboard and the ShareX output file SHALL be left untouched.

#### Scenario: Save three quads
- **WHEN** the image has three valid quads, both folders are enabled and the user presses Ctrl+S
- **THEN** three PNGs are written into each folder and the status reports 3 saved

#### Scenario: Some quads invalid
- **WHEN** one of three quads is degenerate
- **THEN** the two valid quads are saved and the status names the skipped one

### Requirement: Cancel and exit
Esc with no quad being created, or closing the window, SHALL exit without writing further output. In ShareX mode, if no send has happened, the output file SHALL NOT be created, so ShareX continues with the original capture.

#### Scenario: Cancel from ShareX
- **WHEN** in ShareX mode the user presses Esc with no unfinished quad
- **THEN** the app exits and no file exists at the output path

### Requirement: Two configurable output folders
The app SHALL provide two output folder slots, Local and Google Drive. Each has a path chosen with a folder picker and an on/off toggle. A slot with no path SHALL count as off. Both slots SHALL receive identical files.

#### Scenario: Drive slot off
- **WHEN** Local is on, Google Drive is off and the user sends a quad
- **THEN** the file is written only to the Local folder

#### Scenario: Both slots off
- **WHEN** both slots are off and the user sends a quad
- **THEN** the clipboard and the ShareX output still receive the image, and the status notes that no folder was written

### Requirement: Settings persist
Folder paths, folder toggles and the last-used aspect mode and Power of two state SHALL be saved per Windows user. They SHALL be restored on the next start, including when the app is started by ShareX.

#### Scenario: Restart keeps folders
- **WHEN** the user sets the Drive folder to `G:\My Drive\Textures`, closes the app and starts it again
- **THEN** the Drive slot shows `G:\My Drive\Textures` and keeps its toggle state

### Requirement: File naming without overwrite
Files written to output folders SHALL be named `<source-stem>_q<N>.png`. `<source-stem>` is the source file name without extension, or `clipboard_<yyyyMMdd-HHmmss>` for pasted images. `<N>` is the quad's 1-based number in the Quads panel. If the name exists, the app SHALL append `_2`, `_3`, and so on rather than overwrite.

#### Scenario: Name collision
- **WHEN** `shot_q1.png` already exists in the folder and quad 1 of `shot.png` is saved
- **THEN** the file is written as `shot_q1_2.png` and the existing file is unchanged

### Requirement: Unavailable folder does not block other outputs
If an enabled folder cannot be written, the app SHALL still complete the clipboard, the ShareX output and the other folder. A folder may be missing, as with an unmounted `G:\`, or have its writes denied. The failure SHALL be reported with a warning status naming the folder. In ShareX mode a folder failure SHALL keep the app open so the warning is seen; the user closes it.

#### Scenario: Drive not mounted
- **WHEN** the Drive folder is on but `G:\` is not mounted and the user sends a quad in ShareX mode
- **THEN** the clipboard, the output file and the Local folder are written, a warning names the Drive folder, and the app stays open until the user closes it
