## MODIFIED Requirements

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
