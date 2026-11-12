# quad-unwarp Specification

## Purpose

Turns a perspective quad marked on a source image into a frontal, rectangular texture. Sets the rules for the output's size, aspect and sampling quality.

## Requirements

### Requirement: Perspective-correct mapping
The exported texture SHALL be a projective (homography) mapping of the quad onto a rectangle. Each quad corner SHALL map to the matching corner of the output in placement order: first corner to top-left, then clockwise. Straight lines inside the quad SHALL stay straight in the output.

#### Scenario: Rectify a known rectangle
- **GIVEN** a synthetic image of a checkerboard rendered with a known perspective
- **WHEN** a quad is placed exactly on the checkerboard's corners and exported at the board's true size
- **THEN** every square edge in the output is axis-aligned within 1 px

#### Scenario: Corner order
- **WHEN** corners are placed top-left, top-right, bottom-right, bottom-left of a sign
- **THEN** the sign is upright and not mirrored in the output

### Requirement: Reject degenerate quads
A quad that is self-intersecting, or has three nearly collinear corners or a near-zero area, SHALL NOT be exported. It SHALL be marked invalid in the UI.

#### Scenario: Bow-tie quad
- **WHEN** corners are dragged so that two edges cross
- **THEN** the quad is drawn in the danger colour, its preview shows an invalid state and export actions skip it with a status message

### Requirement: Default output size from edges
By default, each quad's output width SHALL be the mean length of its top and bottom edges. Its height SHALL be the mean length of its left and right edges. Both are measured in source pixels and rounded to whole pixels, minimum 1.

#### Scenario: Edge-based size
- **WHEN** a quad's top and bottom edges are 400 and 300 px and its sides are 200 and 200 px
- **THEN** its output size is 350 x 200

### Requirement: Aspect lock
Each quad SHALL have an aspect mode: EDGES (default), 1:1, 4:3, 2:1 or CUSTOM with user-entered width and height ratio terms. With a locked aspect, the output SHALL keep the edge-based width and derive the height from the ratio. The aspect mode SHALL apply to the active quad.

#### Scenario: Lock to 1:1
- **WHEN** the active quad's edge-based size is 350 x 200 and the user picks 1:1
- **THEN** its output size becomes 350 x 350 and the preview updates

#### Scenario: Custom ratio
- **WHEN** the user picks CUSTOM and enters 3 : 1 for a quad with an edge-based width of 600
- **THEN** its output size is 600 x 200

### Requirement: Power-of-two snapping
Each quad SHALL have a Power of two toggle. When on, each output dimension SHALL snap to the nearest power of two (ties round up), applied after the aspect rule, with a minimum of 1.

#### Scenario: Snap
- **WHEN** a quad's size is 350 x 200 and Power of two is on
- **THEN** its output size is 256 x 256

### Requirement: Sampling quality
Export SHALL resample without visible aliasing. Wherever the mapping shrinks the source, several source samples SHALL be averaged per output pixel. Wherever it enlarges, interpolation SHALL be at least bilinear. Alpha in the source SHALL be preserved. Pixels mapping outside the source SHALL be transparent.

#### Scenario: Strong minification
- **WHEN** a quad covering a fine pattern is exported to an output much smaller than its on-image size
- **THEN** the output shows no moire or stair-stepping that is absent from a box-filtered reference

#### Scenario: Quad extends past the image
- **WHEN** a corner lies outside the source bounds
- **THEN** the area mapped from outside the image is fully transparent in the output

### Requirement: Preview matches export geometry
The live preview and the exported file SHALL use the same mapping and the same output aspect. Filtering quality is the only permitted difference.

#### Scenario: Compare preview and export
- **WHEN** the active quad is exported
- **THEN** the exported image has the size shown in the Quads panel and the same framing as the preview
