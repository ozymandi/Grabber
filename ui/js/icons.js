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
