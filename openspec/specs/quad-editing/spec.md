# quad-editing Specification

## Purpose

Lets the user bring a source image into the tool and mark the flat surfaces to extract as perspective quads. The user edits those quads directly on the image and checks the flattened result live.

## Requirements

### Requirement: Load image from command line
When launched with an image path as the first argument, the app SHALL open that image as the source. It SHALL accept PNG, JPEG, BMP and WebP files.

#### Scenario: Launched with a valid path
- **WHEN** the app starts with `ripper.exe "C:\shots\a.png"`
- **THEN** `a.png` is shown on the canvas, fitted to the view

#### Scenario: Launched with a missing or unreadable file
- **WHEN** the first argument points to a file that does not exist or cannot be decoded
- **THEN** the app opens with an empty canvas and the status row shows an error naming the file

### Requirement: Load image interactively
The app SHALL replace the current source image with an image pasted from the clipboard (Ctrl+V or PASTE), a file dropped on the window, or a file chosen with OPEN. Replacing the source SHALL discard the existing quads.

#### Scenario: Paste a screenshot
- **WHEN** the clipboard holds an image and the user presses Ctrl+V
- **THEN** that image becomes the source and the quad list is empty

#### Scenario: Paste with no image in the clipboard
- **WHEN** the user presses Ctrl+V and the clipboard holds no image
- **THEN** the source is unchanged and the status row says there is no image to paste

#### Scenario: Drop a file
- **WHEN** the user drops a supported image file onto the window
- **THEN** that file becomes the source

### Requirement: Navigate the canvas
The canvas SHALL support zooming toward the cursor with the mouse wheel and panning by dragging with the middle mouse button or with Space held. A fit-to-view command SHALL be available.

#### Scenario: Zoom toward cursor
- **WHEN** the user scrolls up over a point of the image
- **THEN** the view zooms in and that image point stays under the cursor

#### Scenario: Pan
- **WHEN** the user holds Space and drags
- **THEN** the view moves with the pointer and the quads move with the image

### Requirement: Create quads
The user SHALL be able to create a quad by clicking its four corners on the image in order. The new quad SHALL become the active quad. Any number of quads SHALL be allowed per image.

#### Scenario: Create a quad with four clicks
- **WHEN** the user clicks four points on the image
- **THEN** a quad with those corners appears, is listed in the Quads panel and is active

#### Scenario: Cancel an unfinished quad
- **WHEN** the user has placed one to three corners and presses Esc
- **THEN** the unfinished quad is discarded and the app does not close

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
