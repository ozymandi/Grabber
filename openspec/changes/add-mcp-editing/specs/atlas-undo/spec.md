## MODIFIED Requirements

### Requirement: What enters the history
A step SHALL be recorded for each content change the user makes in this window: moving, scaling, rotating, moving a pivot, deleting tiles, adding or removing sectors, loading or removing guides, pasting a history state, and applying an LLM proposal. Selection, the active sector, the view, guide Show and Opacity, Fill and Edge, padding and sector size SHALL NOT be recorded. Pictures written by an LLM client and going to a history state SHALL NOT be recorded; History covers them.

#### Scenario: Undo a delete
- **WHEN** the user deletes two tiles and presses Ctrl+Z
- **THEN** both tiles reappear with the same position, size, rotation, pivot and stacking order

#### Scenario: Settings are not steps
- **WHEN** the user moves a tile, then changes Opacity, then presses Ctrl+Z
- **THEN** the tile moves back and Opacity keeps its new value

#### Scenario: Undo an applied proposal
- **WHEN** the user applies an LLM proposal that moves three tiles and presses Ctrl+Z
- **THEN** the three tiles are back where they were

#### Scenario: LLM picture is not undone
- **WHEN** an LLM client writes a picture for a tile and the user presses Ctrl+Z
- **THEN** the picture stays and the status reports nothing to undo
