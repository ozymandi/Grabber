## ADDED Requirements

### Requirement: Picture history of a tile
Every tile SHALL have a history: a stack of its picture states, oldest at the bottom. The first state SHALL be its original picture. A new state SHALL be added only when an LLM client writes a picture for the tile; moving, scaling, rotating and other edits SHALL NOT add states. Each state SHALL keep its full-size picture, a name, its author and its time. States SHALL never be dropped while the tile exists; deleting the tile SHALL remove its history.

#### Scenario: Two LLM edits
- **WHEN** a tile ripped in RIP is restyled by Gemini and then pixelized by Claude
- **THEN** its history holds, from the bottom, the original, the restyled and the pixelized state

#### Scenario: Moves are not states
- **WHEN** the user moves and rotates a tile
- **THEN** its history is unchanged

### Requirement: History of a sector
A sector SHALL have a history of the sector-wide picture edits made on it: one entry per request that wrote pictures to tiles of that sector, naming the request and the tiles it changed. Going to a sector entry SHALL put each of its tiles that still exists into the state that entry wrote. The workspace itself SHALL have no history.

#### Scenario: Undo a sector restyle by history
- **WHEN** sector 1001 was pixelized as one request changing tiles 1, 2 and 3, and the user goes to the entry below it
- **THEN** tiles 1, 2 and 3 show the pictures they had before the pixelize

### Requirement: History panel
ATLAS mode SHALL show a History panel at the top left of the canvas area. It SHALL list the states of the single selected tile, or the entries of the selected sector, newest on top, each with a thumbnail, its name, its author and its time, and SHALL mark the current state. With several tiles, nothing, or only the workspace selected, the panel SHALL say that history is shown for one tile or one sector.

#### Scenario: Select a tile
- **WHEN** the user selects a tile with three states
- **THEN** the panel lists the three states newest on top with the current one marked

### Requirement: Go to a state
Clicking a state SHALL make it the tile's current state: the tile shows that picture in its current frame, at once in the canvas and in every export. States above it SHALL stay in the history. A later picture written by an LLM client SHALL become a new state on top. Going to a state is not an undo step.

#### Scenario: Back to the original
- **WHEN** a tile has the states original, restyled, pixelized and the user clicks the original
- **THEN** the tile shows the original, the panel marks it current, and the restyled and pixelized states are still listed

### Requirement: Copy a state as a new object
A state SHALL be copyable with Ctrl+C or the row's menu and pasteable with Ctrl+V into the atlas. A pasted tile state SHALL become a new tile, unrotated, at the size of the source frame, placed in the active sector by automatic placement. A pasted tile's history SHALL start with the pasted picture as its original. A paste SHALL be one undo step.

#### Scenario: Paste an older state
- **WHEN** the user copies the restyled state of tile 7 and pastes it while sector 1002 is active
- **THEN** a new tile with the restyled picture appears in free space of sector 1002 and its history has one state

#### Scenario: Undo a paste
- **WHEN** the user pastes a state and presses Ctrl+Z
- **THEN** the pasted tile is gone

### Requirement: Copy a sector entry
A copied sector entry SHALL paste all the tiles it changed as new tiles, each with the picture that entry wrote and an unrotated frame of its source size, keeping their layout relative to each other. They SHALL land at the same positions in the active sector when that space is free, otherwise together at the first free spot. The paste SHALL be one undo step.

#### Scenario: Paste a pixelized sector
- **WHEN** the user copies the entry "Pixelize 1001" that changed three tiles and pastes it while the empty sector 1002 is active
- **THEN** three new tiles with the pixelized pictures appear in sector 1002 in the same layout as in 1001
