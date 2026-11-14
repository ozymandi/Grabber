## MODIFIED Requirements

### Requirement: Tile keeps its original
Every tile SHALL keep the full-size image it was created from, in the workspace, as the bottom state of its picture history. The tile shows its current picture state, which is the original until an LLM client writes another. The tile's size on the canvas is a scale of the current picture. Every resize and every export SHALL be rendered from the current full-size picture, never from an earlier rendered copy.

#### Scenario: Shrink and enlarge again
- **WHEN** a 1600 x 900 tile is scaled down to 200 x 113 and later scaled back to 1600 x 900
- **THEN** the exported tile matches the original pixel for pixel

#### Scenario: Export after an LLM edit
- **WHEN** an LLM client writes a new picture for a tile and the user exports its sector
- **THEN** the sector shows the new picture in the tile's frame, and going back to the original in History brings the original back into the next export
