// Marquee selection in ATLAS (design D3 of add-atlas-marquee). Pure: no DOM, so
// `marquee.test.mjs` runs it under node.

const EPS = 1e-9;

/** Box { x0, y0, x1, y1 } spanned by two canvas points, in any order. */
export function boxFrom(a, b) {
    return { x0: Math.min(a[0], b[0]), y0: Math.min(a[1], b[1]), x1: Math.max(a[0], b[0]), y1: Math.max(a[1], b[1]) };
}

/** Min and max of the points projected on `axis`. */
function span(points, axis) {
    let lo = Infinity;
    let hi = -Infinity;
    for (const p of points) {
        const v = p[0] * axis[0] + p[1] * axis[1];
        if (v < lo) lo = v;
        if (v > hi) hi = v;
    }
    return [lo, hi];
}

/**
 * True when the box touches the tile outline given by its 4 corners (in order around the
 * outline). Separating axes: the box's x and y and the outline's two edge normals; shared
 * edges and corners count as touching.
 */
export function touches(corners, box) {
    const quad = [
        [box.x0, box.y0],
        [box.x1, box.y0],
        [box.x1, box.y1],
        [box.x0, box.y1],
    ];
    const e0 = [corners[1][0] - corners[0][0], corners[1][1] - corners[0][1]];
    const e1 = [corners[2][0] - corners[1][0], corners[2][1] - corners[1][1]];
    const axes = [[1, 0], [0, 1], [-e0[1], e0[0]], [-e1[1], e1[0]]];
    for (const axis of axes) {
        const len = Math.hypot(axis[0], axis[1]);
        if (len === 0) continue;
        const n = [axis[0] / len, axis[1] / len];
        const a = span(corners, n);
        const b = span(quad, n);
        if (a[1] < b[0] - EPS || b[1] < a[0] - EPS) return false;
    }
    return true;
}
