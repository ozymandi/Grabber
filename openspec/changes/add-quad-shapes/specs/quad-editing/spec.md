## MODIFIED Requirements

### Requirement: Edit quad corners
The corners of the active quad SHALL be shown as handles and SHALL be draggable, and a drag SHALL update the quad immediately. Inactive quads SHALL show no corner handles. Corner handles SHALL keep a constant on-screen size at any zoom level. Corner positions SHALL be stored in source-image pixel coordinates with sub-pixel precision.

#### Scenario: Drag a corner
- **WHEN** the user drags a corner handle of the active quad
- **THEN** that corner follows the pointer

#### Scenario: Corners of an inactive quad
- **WHEN** a quad is not active
- **THEN** its corners show no handles, and a press on such a corner acts on what lies under it

#### Scenario: Handles at high zoom
- **WHEN** the view is zoomed in 8x
- **THEN** the corner handles are drawn at the same screen size as at 1x

### Requirement: Active quad and removal
Exactly one quad SHALL be active whenever at least one quad exists. The user SHALL be able to activate a quad by clicking inside its shape or its row in the Quads panel. The user SHALL be able to delete a quad with Delete or the row's remove icon.

#### Scenario: Activate from the list
- **WHEN** the user clicks the second row in the Quads panel
- **THEN** the second quad becomes active and its edges and corner handles appear on the canvas

#### Scenario: Activate by its shape
- **WHEN** the user clicks inside the circle of an inactive Circle quad
- **THEN** that quad becomes active

#### Scenario: Delete the active quad
- **WHEN** the active quad is deleted and other quads remain
- **THEN** another quad becomes active

### Requirement: Live preview of the active quad
The Preview panel SHALL show the active quad flattened to a frontal view using its current size settings, with its shape's mask: outside the shape the transparency checker shows. The preview SHALL update while corners, anchors or handles are dragged, without perceptible lag at 4K source resolution.

#### Scenario: Preview follows a drag
- **WHEN** the user drags a corner of the active quad on a 3840x2160 source
- **THEN** the preview updates continuously during the drag

#### Scenario: Preview of a circle
- **WHEN** the active quad's shape is Circle
- **THEN** the preview shows the circle with the checker in its corners

#### Scenario: No quads
- **WHEN** the image has no quads
- **THEN** the Preview panel shows an empty state
