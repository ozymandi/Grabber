# atlas-undo Specification

## Purpose
Defines undo and redo of the user's own edits in ATLAS mode: what enters the history, how long it lives, and how undo behaves when the workspace was changed from elsewhere in the meantime.

## Requirements

### Requirement: Undo and redo keys
In ATLAS mode Ctrl+Z SHALL undo the last step, and Ctrl+Shift+Z or Ctrl+Y SHALL redo the last undone step. The keys SHALL work on any keyboard layout. The status SHALL name the step undone or redone. With nothing to undo or redo the keys SHALL do nothing but say so in the status. RIP mode SHALL have no undo.

#### Scenario: Undo a move
- **WHEN** the user drags three tiles and presses Ctrl+Z
- **THEN** the three tiles are back where they were and the status names the undone move

#### Scenario: Cyrillic layout
- **WHEN** the keyboard layout is Ukrainian and the user presses Ctrl and the physical Z key
- **THEN** the last step is undone

### Requirement: What enters the history
A step SHALL be recorded for each content change the user makes in this window: moving, scaling, rotating, moving a pivot, deleting tiles, adding or removing sectors, loading or removing guides. Selection, the active sector, the view, guide Show and Opacity, Fill and Edge, padding and sector size SHALL NOT be recorded.

#### Scenario: Undo a delete
- **WHEN** the user deletes two tiles and presses Ctrl+Z
- **THEN** both tiles reappear with the same position, size, rotation, pivot and stacking order

#### Scenario: Settings are not steps
- **WHEN** the user moves a tile, then changes Opacity, then presses Ctrl+Z
- **THEN** the tile moves back and Opacity keeps its new value

### Requirement: One gesture is one step
Each gesture SHALL be one step: one drag of any number of tiles, one handle drag, one rotation drag, one R press, one pivot drag or reset, one Delete, one sector added or removed, one LOAD or REMOVE. A LOAD that created sectors SHALL undo as one step that removes the loaded guides and the created sectors and restores any guides they replaced.

#### Scenario: Undo a set load
- **WHEN** loading `body.1001.png` to `body.1004.png` created three sectors and the user presses Ctrl+Z
- **THEN** the guides are gone and the workspace has only its original sector

### Requirement: Tiles from outside are not undone
Tiles added by a ShareX launch or by SEND and SAVE ALL in RIP mode SHALL NOT be steps. Undo SHALL never remove such a tile.

#### Scenario: Capture arrives between edits
- **WHEN** the user moves a tile, a ShareX capture adds a tile, and the user presses Ctrl+Z twice
- **THEN** the move is undone, the captured tile stays, and the second Ctrl+Z reports nothing to undo

### Requirement: History lifetime
The history SHALL belong to the window and hold up to 100 steps; older steps are dropped. It SHALL survive SAVE and SAVE AS. NEW and OPEN SHALL start an empty history. Closing the window SHALL discard the history and keep every change. A new step SHALL clear the redo steps.

#### Scenario: Undo after saving
- **WHEN** the user deletes a tile, saves, and presses Ctrl+Z
- **THEN** the tile reappears and the workspace is marked as having unsaved changes

#### Scenario: Open starts fresh
- **WHEN** the user moves a tile and then opens another `.ripper` file
- **THEN** Ctrl+Z reports nothing to undo and does not touch either workspace

### Requirement: Undo marks unsaved changes
Every undo and redo SHALL count as a change: it is written to the working copy at once and the workspace title shows the unsaved mark, even when the result equals the saved file.

#### Scenario: Back to the saved state
- **WHEN** the user saves, moves a tile and undoes the move
- **THEN** the workspace still shows the unsaved mark

### Requirement: Conflicts are skipped
Undo and redo SHALL reverse only the step's own changes, applied to the workspace as it is now. A part that cannot be applied, such as a tile deleted elsewhere or a sector that now holds a tile, SHALL be skipped and named in the status, and the step SHALL still be consumed. Tiles added from elsewhere SHALL be kept.

#### Scenario: Sector received a tile
- **WHEN** the user adds sector 1002, a ShareX tile is placed in it, and the user presses Ctrl+Z
- **THEN** sector 1002 and its tile stay, the status explains that the sector was kept because a tile arrived there, and the next Ctrl+Z undoes the step before it

#### Scenario: Tile deleted in another window
- **WHEN** the user moves a tile, another ripper window deletes that tile, and the user presses Ctrl+Z
- **THEN** nothing else changes and the status reports that the tile no longer exists
