// SVG icons generated from JS. Glyphs are the O'Tool originals:
// remove = O'Frame slider reset cross (12px box, stroke 1.4).
// update = the designer's icons/SVG/Update.svg, geometry, viewBox and stroke width as drawn.
//   Named deviations: background gradient rect dropped (the button is transparent, like
//   Help), #fff -> currentColor (muted / hover colours like Help), classes inlined (an
//   inline <style> would leak into the page).

export const REMOVE_ICON = `<svg viewBox="0 0 12 12" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
    <path d="M3 3L9 9M9 3L3 9" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
</svg>`;

export const UPDATE_ICON = `<svg viewBox="0 0 1024 1024" fill="none" stroke="currentColor" stroke-width="46" stroke-miterlimit="10" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
    <polygon points="206.56 399.68 206.56 744.32 279.68 744.32 279.68 817.44 744.32 817.44 744.32 744.32 817.44 744.32 817.44 279.68 744.32 279.68 744.32 220.92 279.68 312.2 279.68 399.68 206.56 399.68"/>
    <rect x="744.32" y="744.32" width="146.23" height="146.23"/>
    <rect x="744.32" y="133.45" width="146.23" height="146.23"/>
    <rect x="133.45" y="744.32" width="146.23" height="146.23"/>
    <rect x="133.45" y="253.45" width="146.23" height="146.23"/>
    <line x1="512" y1="380.46" x2="512" y2="643.54"/>
    <polyline points="599.71 559.66 512 643.54 424.29 559.66"/>
</svg>`;

// Rotate cursor (add-atlas-fine-work): Lucide `rotate-cw` (lucide-static 1.72.0, ISC), paths
// as published. A CSS cursor cannot read CSS variables, so the stroke is the literal --ink
// (#ffffff) over a --bg (#1e1e1e) halo for contrast on any texture: a named deviation.
const ROTATE_PATHS = ['M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8', 'M21 3v5h-5'];
const rotateSvg = `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke-linecap="round" stroke-linejoin="round">${ROTATE_PATHS.map(
    (d) => `<path d="${d}" stroke="#1e1e1e" stroke-width="4"/>`,
).join('')}${ROTATE_PATHS.map((d) => `<path d="${d}" stroke="#ffffff" stroke-width="2"/>`).join('')}</svg>`;
export const ROTATE_CURSOR = `url("data:image/svg+xml,${encodeURIComponent(rotateSvg)}") 12 12, crosshair`;

// Pivot marker: Lucide `crosshair` (lucide-static 1.72.0, ISC) geometry on its 24 grid, drawn
// on the ATLAS canvas (stroke 2 on that grid, round caps).
export const CROSSHAIR = {
    circle: [12, 12, 10],
    lines: [
        [22, 12, 18, 12],
        [6, 12, 2, 12],
        [12, 6, 12, 2],
        [12, 22, 12, 18],
    ],
};

// Sector "+" glyph: Lucide `plus` (lucide-static 1.72.0, ISC), as published; only the fixed
// width/height are dropped so the button sizes it. O'Tool has no plus glyph; the designer
// chose Lucide for new icons.
export const PLUS_ICON = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
    <path d="M5 12h14"/>
    <path d="M12 5v14"/>
</svg>`;

// Quad shape icons (add-quad-shapes D9): Lucide `square`, `circle`, `octagon` and `pen-tool`
// (lucide-static 1.72.0, ISC), paths as published; only the fixed width/height and the class
// are dropped so the button sizes them.
const lucide = (body) =>
    `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">${body}</svg>`;
export const SHAPE_ICONS = {
    rect: lucide('<rect width="18" height="18" x="3" y="3" rx="2"/>'),
    circle: lucide('<circle cx="12" cy="12" r="10"/>'),
    ngon: lucide(
        '<path d="M2.586 16.726A2 2 0 0 1 2 15.312V8.688a2 2 0 0 1 .586-1.414l4.688-4.688A2 2 0 0 1 8.688 2h6.624a2 2 0 0 1 1.414.586l4.688 4.688A2 2 0 0 1 22 8.688v6.624a2 2 0 0 1-.586 1.414l-4.688 4.688a2 2 0 0 1-1.414.586H8.688a2 2 0 0 1-1.414-.586z"/>',
    ),
    path: lucide(
        '<path d="m18 13-.654-4.736a4 4 0 00-2.756-3.266l-9.163-2.9a2 2 0 00-2.017.493l-.82.819a2 2 0 00-.492 2.017l2.9 9.163a4 4 0 003.266 2.756L13 18"/><path d="M18.648 12.352a1.205 1.205 0 011.704 0l1.296 1.296a1.205 1.205 0 010 1.704l-6.296 6.296a1.205 1.205 0 01-1.704 0l-1.296-1.296a1.205 1.205 0 010-1.704z"/><path d="m3 3 6.586 6.586"/><circle cx="11" cy="11" r="2"/>',
    ),
};

// Pen cursors (add-quad-shapes, "Pen cursors"): Lucide `pen-tool`, `plus`, `minus`, `circle`,
// `chevron-up` and `mouse-pointer-2` (lucide-static 1.72.0, ISC), paths as published. Like the
// rotate cursor, a CSS cursor cannot read CSS variables, so strokes are the literal --ink
// (#ffffff) over a --bg (#1e1e1e) halo: a named deviation. The badge (+, -, o) is the Lucide
// glyph at 12 px beside the pen; its strokes are doubled on its 24 grid so it draws as wide as
// the pen. Strokes are 1 px (half of Lucide's 2, at the designer's request) with a 1 px halo
// at 20% opacity (the designer's request). All halos are drawn first, as one group so overlaps
// do not darken, and no halo covers a white stroke.
const PEN = [
    '<path d="m18 13-.654-4.736a4 4 0 00-2.756-3.266l-9.163-2.9a2 2 0 00-2.017.493l-.82.819a2 2 0 00-.492 2.017l2.9 9.163a4 4 0 003.266 2.756L13 18"/>',
    '<path d="M18.648 12.352a1.205 1.205 0 011.704 0l1.296 1.296a1.205 1.205 0 010 1.704l-6.296 6.296a1.205 1.205 0 01-1.704 0l-1.296-1.296a1.205 1.205 0 010-1.704z"/>',
    '<path d="m3 3 6.586 6.586"/>',
    '<circle cx="11" cy="11" r="2"/>',
].join('');
const BADGES = {
    plus: '<path d="M5 12h14"/><path d="M12 5v14"/>',
    minus: '<path d="M5 12h14"/>',
    close: '<circle cx="12" cy="12" r="10"/>',
};
const ARROW = '<path d="M4.037 4.688a.495.495 0 0 1 .651-.651l16 6.5a.5.5 0 0 1-.063.947l-6.124 1.58a2 2 0 0 0-1.438 1.435l-1.579 6.126a.5.5 0 0 1-.947.063z"/>';
const CARET = '<path d="m18 15-6-6-6 6"/>';
const layer = (body, color, width) => `<g stroke="${color}" stroke-width="${width}">${body}</g>`;
const badgeAt = (body, color, width) =>
    `<svg x="19" y="1" width="12" height="12" viewBox="0 0 24 24" overflow="visible">${layer(body, color, width)}</svg>`;
function cursorSvg(glyph, badge) {
    const b = badge ? BADGES[badge] : null;
    return `<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 32 32" fill="none" stroke-linecap="round" stroke-linejoin="round"><g opacity="0.2">${layer(glyph, '#1e1e1e', 3)}${
        b ? badgeAt(b, '#1e1e1e', 6) : ''
    }</g>${layer(glyph, '#ffffff', 1)}${b ? badgeAt(b, '#ffffff', 2) : ''}</svg>`;
}
/** The six cursor pictures, with their hot spots (the pen's nib, the arrow's tip, the caret's apex). */
export const PEN_CURSOR_SVGS = {
    'pen-add': { svg: cursorSvg(PEN, 'plus'), hot: [2, 2] },
    'pen-remove': { svg: cursorSvg(PEN, 'minus'), hot: [2, 2] },
    'pen-close': { svg: cursorSvg(PEN, 'close'), hot: [2, 2] },
    pen: { svg: cursorSvg(PEN, null), hot: [2, 2] },
    convert: { svg: cursorSvg(CARET, null), hot: [12, 9] },
    direct: { svg: cursorSvg(ARROW, null), hot: [4, 4] },
};
/** CSS cursor values, keyed like PEN_CURSOR_SVGS. */
export const PEN_CURSORS = Object.fromEntries(
    Object.entries(PEN_CURSOR_SVGS).map(([k, { svg, hot }]) => [
        k,
        `url("data:image/svg+xml,${encodeURIComponent(svg)}") ${hot[0]} ${hot[1]}, crosshair`,
    ]),
);
