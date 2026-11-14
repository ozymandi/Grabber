## ADDED Requirements

### Requirement: Connect an LLM client
ripper SHALL ship an MCP server, `ripper-mcp.exe`, next to `ripper.exe`. An MCP client such as Claude Code, Codex or Gemini CLI SHALL connect to it over stdio with only the program path in its configuration. The server SHALL work on the current workspace whether or not a ripper window is open. It SHALL use no network and SHALL never edit pictures itself: the client edits them with its own means.

#### Scenario: Connect Claude Code
- **WHEN** the user adds `%LOCALAPPDATA%\ripper\ripper-mcp.exe` as an MCP server in Claude Code and asks it to list the atlas
- **THEN** it gets the current workspace's name, sectors and tiles

#### Scenario: No window open
- **WHEN** no ripper window is open and the client asks for tile 7
- **THEN** the server answers from the workspace on disk

### Requirement: Read the atlas
The server SHALL let the client read the current workspace: its name, sector size, sectors, active sector and every tile's number, sector, frame, picture size and provenance. For a tile it SHALL give the path of a full-size PNG of the current picture and a thumbnail at most 256 pixels on its long side. For a sector it SHALL render the sector to a PNG at full size and give its path.

#### Scenario: Look at a tile
- **WHEN** the client asks for tile 7, a 2048 x 1024 picture
- **THEN** it gets a PNG path with the 2048 x 1024 picture and a 256 x 128 thumbnail

### Requirement: Scope follows the selection
Without an explicit target, a request SHALL work on the selection of the open ATLAS window. Selected tiles come first, then selected sectors, which mean all their tiles. With nothing selected or no window open, the scope is the whole atlas, and the client SHALL be told to ask the user before writing. A target named in the request SHALL win over the selection. The scope SHALL be fixed when the client reads it.

#### Scenario: Three tiles selected
- **WHEN** tiles 2, 5 and 9 are selected in ATLAS and the user asks the LLM to pixelize them
- **THEN** the scope is exactly tiles 2, 5 and 9

#### Scenario: Nothing selected
- **WHEN** nothing is selected and the client reads the scope
- **THEN** the scope is every tile in the atlas and is marked as needing the user's confirmation

#### Scenario: Selection changes mid-task
- **WHEN** the client has read a scope of two tiles and the user then selects another tile
- **THEN** the task still works on the two tiles

### Requirement: Write a tile picture
The client SHALL write a new picture for a tile from a PNG path. The tile's frame SHALL stay. The picture SHALL become the tile's current state in its history, named by the client, with the client's name and the time. Unless the client asks otherwise, the alpha of the tile's original picture SHALL be applied to the new picture, scaled to its size. A picture of another size SHALL be allowed.

#### Scenario: Pixelized circle tile
- **WHEN** the client writes a 64 x 64 opaque picture for a 512 x 512 Circle tile
- **THEN** the tile shows the 64 x 64 picture in its old frame, outside the circle stays transparent, and its history has a new state on top

#### Scenario: Open window updates
- **WHEN** a ripper window shows the workspace in ATLAS and the client writes a tile picture
- **THEN** the window shows the new picture within a second without being reloaded

### Requirement: Sector edits per tile or whole
For a sector or the whole atlas the client SHALL choose per request between editing each tile on its own and editing one render of the sector. A sector render written back SHALL be cut into the sector's tiles by their frames, each tile getting a new state. A tile turned by other than whole quarter turns, or not wholly inside the sector, SHALL be refused in a whole-sector write and named; the others SHALL be written.

#### Scenario: Whole sector restyled
- **WHEN** the client renders sector 1001 with three upright tiles, restyles the render and writes it back
- **THEN** each of the three tiles gets a new state holding its part of the restyled render

#### Scenario: Tilted tile in the sector
- **WHEN** sector 1002 holds a tile turned by 30 degrees and the client writes a whole-sector render
- **THEN** the upright tiles get new states and the reply names the tilted tile as not written

### Requirement: Layout changes are proposals
The client SHALL NOT move, scale, rotate or delete tiles, change sectors or export directly. It SHALL write a proposal listing such changes. An open ATLAS window SHALL show the proposal as a preview over the canvas with APPLY and DISCARD. APPLY SHALL make the changes as one step; DISCARD SHALL drop them. A newer proposal SHALL replace an earlier one that was not applied.

#### Scenario: Proposed move
- **WHEN** the client proposes moving tile 4 into sector 1002
- **THEN** ATLAS shows tile 4 at the new place as a preview, and only APPLY moves it

#### Scenario: Discard
- **WHEN** the user presses DISCARD on a proposal
- **THEN** nothing in the workspace changes and the preview disappears

### Requirement: Safe with an open window
Every write by the server SHALL keep the workspace consistent with an open window and with ShareX captures: no change from either side SHALL be lost. A write that cannot get the workspace in time SHALL fail with a clear message and change nothing.

#### Scenario: Capture during an edit
- **WHEN** a ShareX capture adds a tile while the client writes a picture for tile 3
- **THEN** the workspace has both the new tile and the new picture of tile 3
