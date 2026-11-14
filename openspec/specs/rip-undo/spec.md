# rip-undo Specification

## Purpose
Defines undo and redo of quad edits in RIP mode: which edits enter the history, how many steps it keeps, and what undo leaves alone.

## Requirements

### Requirement: Undo quad edits in RIP
In RIP mode, Ctrl+Z SHALL undo the last change to the quads, and Ctrl+Shift+Z or Ctrl+Y SHALL redo it. Changes are: creating or removing a quad, a corner drag, a shape or N change, a path edit or Pen drawing, and the active quad's aspect or power of two. One drag SHALL be one step. Undo and redo SHALL restore which quad is active. The history SHALL keep at least 100 steps, in memory only.

#### Scenario: Undo a corner drag
- **WHEN** the user drags a corner and presses Ctrl+Z
- **THEN** the corner is back where it was and the preview matches

#### Scenario: Undo a removal
- **WHEN** the user deletes quad Q2 of three and presses Ctrl+Z
- **THEN** Q2 is back in its place in the list, with its shape, and is active

#### Scenario: Undo a path edit
- **WHEN** the user Alt+drags handles out of a path anchor and presses Ctrl+Z
- **THEN** the anchor is a corner again with no handles

#### Scenario: Undo an aspect change
- **WHEN** the user switches the active quad from EDGES to 1:1 and presses Ctrl+Z
- **THEN** the quad is back to EDGES and the preview has its earlier size

#### Scenario: Redo
- **WHEN** the user undoes a shape change and presses Ctrl+Y
- **THEN** the shape change is applied again

#### Scenario: New edit clears redo
- **WHEN** the user undoes a step and then moves a corner
- **THEN** Ctrl+Y does nothing

### Requirement: What undo leaves alone
Undo SHALL NOT revert files written by Send or Save All, the clipboard, output folders, Atlas output, or other settings. A quad changed by undo or redo SHALL count as unsaved. Loading a new source image by OPEN, PASTE, drop or ShareX launch SHALL clear the history.

#### Scenario: Undo after Send
- **WHEN** the user sends a quad and then undoes its last corner drag
- **THEN** the written file stays, and the quad is marked unsaved

#### Scenario: New image clears the history
- **WHEN** the user pastes a new image and presses Ctrl+Z
- **THEN** nothing changes
