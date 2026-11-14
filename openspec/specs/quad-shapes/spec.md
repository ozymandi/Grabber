# quad-shapes Specification

## Purpose
Defines the shape a RIP quad cuts out of its warped plane (Rectangle, Circle, regular N-gon or a Bezier Path), how that shape masks every output, how it is chosen and drawn on the canvas, and how a Path is edited with Pen-tool gestures and cursors.

## Requirements

### Requirement: Quad shapes
Every quad SHALL have a shape: Rectangle, Circle, N-gon or Path. The quad SHALL still define the plane, the homography and the output size, and the shape SHALL be a mask on that plane. Rectangle SHALL cover the whole output. Circle SHALL be the ellipse inscribed in the output rectangle. N-gon SHALL be a regular polygon with N from 3 to 12, scaled to touch all four sides of the output: for N = 3 with a vertex at the top, and for even N with a flat top side.

#### Scenario: Circle on a square output
- **GIVEN** a quad with a 512 x 512 output and shape Circle
- **WHEN** it is exported
- **THEN** the PNG is 512 x 512, its pixels inside the circle of radius 256 about the centre are opaque, and its corners are transparent

#### Scenario: Octagon on a STOP sign
- **GIVEN** a quad whose corners lie on the square around a STOP sign in the photo, with shape N-gon and N = 8
- **WHEN** it is exported
- **THEN** the octagon's top, bottom, left and right sides lie on the edges of the output and its four slanted corners are transparent

#### Scenario: Triangle points up
- **WHEN** a quad has shape N-gon with N = 3
- **THEN** one vertex is at the middle of the output's top edge and the opposite side lies on the bottom edge

#### Scenario: Rectangle is unchanged
- **WHEN** a quad has shape Rectangle
- **THEN** the export is pixel-identical to the export of the same quad before this change

### Requirement: Mask in every output
The shape SHALL apply to the Preview and to every output of a quad: Send, Copy, Save All, the ShareX output and Atlas tiles. Pixels outside the shape SHALL be transparent, and pixels on its edge SHALL get partial alpha from their covered area. The clipboard SHALL carry the alpha. Size, aspect and power of two SHALL be computed as for a rectangle.

#### Scenario: Copy a circle
- **WHEN** the user copies a Circle quad and pastes it into an editor that reads PNG from the clipboard
- **THEN** the pasted image has transparent corners

#### Scenario: Smooth edge
- **WHEN** a Circle quad is exported
- **THEN** pixels crossed by the circle's edge have alpha strictly between 0 and 255

#### Scenario: Atlas tile with EXTEND
- **GIVEN** the Atlas output is on and the workspace uses Fill EXTEND
- **WHEN** a Circle quad is sent and its sector is exported
- **THEN** the tile's transparent corners within Edge pixels take the colour of the nearest circle pixel

### Requirement: Choose a shape
The header of the Quads panel SHALL show an icon of the active quad's shape. When no quad exists, the icon SHALL show the shape for new quads. Tapping the icon SHALL open a popup listing Rectangle, Circle, N-gon with a field for N, and Path. Choosing one SHALL set the shape of the active quad and the shape for new quads. Shift+S SHALL cycle the active quad's shape through Rectangle, Circle, N-gon and Path. The shape and N for new quads SHALL be kept in the settings across launches.

#### Scenario: Pick Circle in the popup
- **GIVEN** the active quad is a Rectangle
- **WHEN** the user taps the shape icon and chooses Circle
- **THEN** the active quad becomes a Circle, the icon shows a circle and the preview shows the circle

#### Scenario: Cycle with Shift+S
- **WHEN** the active quad is a Circle and the user presses Shift+S
- **THEN** the active quad becomes an N-gon with the N last used

#### Scenario: New quads take the last shape
- **GIVEN** the user chose N-gon with N = 8 and restarted the app
- **WHEN** the user creates a quad
- **THEN** the new quad is an octagon

### Requirement: Marching-ants outlines
Every quad's shape outline SHALL be drawn on the canvas as marching ants: a 1 CSS-pixel line of alternating black and white 4-pixel dashes that keep moving. The quad itself, its edges and corner handles, SHALL be drawn only for the active quad. A click inside an inactive quad's shape SHALL make that quad active.

#### Scenario: Two signs
- **GIVEN** two quads with shape Circle, the second active
- **WHEN** the user looks at the canvas
- **THEN** both circles show marching ants, and only the second quad shows its edges and corner handles

#### Scenario: Click beside a circle
- **GIVEN** an inactive Circle quad
- **WHEN** the user clicks inside its quad but outside its circle
- **THEN** that quad does not become active and the click starts a new quad

### Requirement: Path shape
A Path SHALL be one closed path of cubic Bézier segments, with at least 2 anchors, stored in the quad's own coordinates so that it moves with the quad when corners move. Each anchor SHALL be a corner or smooth. A smooth anchor's two handles SHALL lie on one line through the anchor. Switching a quad to Path SHALL start from its current shape: Rectangle gives 4 corner anchors at the corners, Circle gives 4 smooth anchors on the ellipse with handles that trace it, and N-gon gives N corner anchors.

#### Scenario: Path follows the quad
- **GIVEN** a Path quad
- **WHEN** the user drags one of the quad's corners
- **THEN** the path stays in the same place relative to the quad, and its export is unchanged except for the new perspective

#### Scenario: Circle to Path
- **WHEN** a Circle quad is switched to Path
- **THEN** the path has 4 smooth anchors, and its export differs from the circle's by at most 1 alpha level per 255 at any pixel

### Requirement: Pen adds and removes anchors
On the active quad's path, as with Illustrator's Pen tool, a click on a segment SHALL add an anchor there without changing the curve, and a click on an anchor SHALL remove it unless only 2 anchors remain.

#### Scenario: Add an anchor on a curve
- **WHEN** the user clicks on a curved segment of the active path
- **THEN** a new anchor appears at that point and the outline does not change shape

#### Scenario: Remove an anchor
- **WHEN** the active path has 5 anchors and the user clicks one of them
- **THEN** the path has 4 anchors and passes through the other four

#### Scenario: Keep two anchors
- **WHEN** the active path has 2 anchors and the user clicks one of them
- **THEN** the path keeps both anchors

### Requirement: Pen moves anchors and handles
On the active quad's path, as with Illustrator's Pen tool, Ctrl+drag SHALL move an anchor with its handles, or move one handle. Dragging a smooth anchor's handle SHALL keep its partner pointing the opposite way. Shift SHALL hold a drag to 45-degree steps in the quad's plane.

#### Scenario: Move an anchor
- **WHEN** the user Ctrl+drags an anchor
- **THEN** the anchor and its handles move together and the preview follows

#### Scenario: Turn a smooth anchor's handle
- **WHEN** the user Ctrl+drags one handle of a smooth anchor
- **THEN** the other handle turns with it and stays on the opposite side of the anchor

### Requirement: Pen converts anchors
On the active quad's path, as with Illustrator's Pen tool with Alt held, Alt+drag from an anchor SHALL pull out symmetric handles and make it smooth, Alt+click on a smooth anchor SHALL make it a corner, and Alt+drag on a handle SHALL move it independently of its partner.

#### Scenario: Make a corner
- **WHEN** the user Alt+clicks a smooth anchor
- **THEN** its handles disappear and the adjacent segments meet it at a sharp corner

#### Scenario: Pull out handles
- **WHEN** the user Alt+drags from a corner anchor
- **THEN** two handles appear on a line through the anchor, mirrored about it, and the adjacent segments curve

#### Scenario: Break a handle
- **WHEN** the user Alt+drags one handle of a smooth anchor
- **THEN** only that handle moves and the other keeps its place

### Requirement: Draw a path with the Pen
With a Path quad active, P SHALL start drawing a new path for it with the Pen. While drawing:
- a click SHALL add a corner anchor;
- a click-drag SHALL add a smooth anchor whose handles follow the drag;
- a click on the first anchor SHALL close the path and finish.

Esc while drawing SHALL cancel it, keep the previous path and not close the app. A drawing with fewer than 2 anchors SHALL NOT replace the previous path.

#### Scenario: Draw a teardrop
- **WHEN** the user presses P, click-drags one anchor, clicks two more and clicks the first anchor
- **THEN** the quad's path has 3 anchors, the first one smooth, and the preview shows the new shape

#### Scenario: Esc while drawing
- **WHEN** the user presses P, adds two anchors and presses Esc
- **THEN** the previous path is back, and the app stays open in standalone and ShareX mode

### Requirement: Pen cursors
Over the RIP canvas the cursor SHALL show what a click or drag on the active path would do, as with Illustrator's Pen tool:
- a pen with "+" over a segment;
- a pen with "-" over an anchor;
- a caret with Alt held over an anchor or handle;
- an arrow with Ctrl held, or over a handle;
- while drawing with P, a pen, and a pen with "o" over the first anchor.

Ctrl and Alt SHALL change it without moving the mouse. Corners, panning and new quads keep their cursors.

#### Scenario: Hold Alt over an anchor
- **WHEN** the pointer rests on an anchor of the active path and the user presses Alt
- **THEN** the cursor turns from the pen with "-" to the caret, and back on release

#### Scenario: Close with the Pen
- **WHEN** the user draws with P and the pointer reaches the first anchor after two or more
- **THEN** the cursor is the pen with "o"
