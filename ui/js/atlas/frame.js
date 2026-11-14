// Tile frame geometry, mirroring `Frame` in src-tauri/src/atlas/model.rs (design D1 of
// add-atlas-fine-work). A frame is any object with the tile fields
// { c2: [x2, y2], w, h, rot, tilt, pivot }: the centre in half pixels, the size before
// rotation, quarter turns, extra degrees and the pivot in whole pixels from the unrotated
// top-left (null = centre). Rust stays authoritative; these give the same numbers for drawing,
// hit tests and drag previews.

export const angle = (f) => f.rot * 90 + (f.tilt || 0);

export const centre = (f) => [f.c2[0] / 2, f.c2[1] / 2];

/** Rotates a vector clockwise on screen (y down) by `deg`. */
export function turnV(v, deg) {
    const a = (deg * Math.PI) / 180;
    const c = Math.cos(a);
    const s = Math.sin(a);
    return [v[0] * c - v[1] * s, v[0] * s + v[1] * c];
}

/** Pivot offset from the centre in the unrotated frame. */
export const pivotLocal = (f) => (f.pivot ? [f.pivot[0] - f.w / 2, f.pivot[1] - f.h / 2] : [0, 0]);

/** Canvas point of an offset from the centre in the unrotated frame. */
export function toCanvas(f, l) {
    const v = turnV(l, angle(f));
    const c = centre(f);
    return [c[0] + v[0], c[1] + v[1]];
}

/** Offset from the centre in the unrotated frame of a canvas point. */
export function toLocal(f, p) {
    const c = centre(f);
    return turnV([p[0] - c[0], p[1] - c[1]], -angle(f));
}

export const pivotCanvas = (f) => toCanvas(f, pivotLocal(f));

/** Local offsets of the unrotated corners, in the order of `CORNERS`. */
export const CORNERS = ['top_left', 'top_right', 'bottom_right', 'bottom_left'];
export const OPPOSITE = { top_left: 'bottom_right', top_right: 'bottom_left', bottom_right: 'top_left', bottom_left: 'top_right' };

export function cornerLocal(f, name, w = f.w, h = f.h) {
    const x = name.endsWith('left') ? -w / 2 : w / 2;
    const y = name.startsWith('top') ? -h / 2 : h / 2;
    return [x, y];
}

export const corners = (f) => CORNERS.map((n) => toCanvas(f, cornerLocal(f, n)));

/** Bounding box in whole pixels, as Rust computes it. */
export function rect(f) {
    if (!f.tilt) {
        const [rw, rh] = f.rot % 2 === 1 ? [f.h, f.w] : [f.w, f.h];
        return { x: Math.floor((f.c2[0] - rw) / 2), y: Math.floor((f.c2[1] - rh) / 2), w: rw, h: rh };
    }
    const cs = corners(f);
    const x0 = Math.floor(Math.min(...cs.map((p) => p[0])));
    const y0 = Math.floor(Math.min(...cs.map((p) => p[1])));
    const x1 = Math.ceil(Math.max(...cs.map((p) => p[0])));
    const y1 = Math.ceil(Math.max(...cs.map((p) => p[1])));
    return { x: x0, y: y0, w: Math.max(1, x1 - x0), h: Math.max(1, y1 - y0) };
}

/** True when the canvas point lies on the tile itself (not just in its bounding box). */
export function contains(f, p) {
    const l = toLocal(f, p);
    return Math.abs(l[0]) <= f.w / 2 && Math.abs(l[1]) <= f.h / 2;
}

const frameOf = (f) => ({ c2: [...f.c2], w: f.w, h: f.h, rot: f.rot, tilt: f.tilt || 0, pivot: f.pivot ? [...f.pivot] : null });

function setCentre(f, c) {
    f.c2 = [Math.round(2 * c[0]), Math.round(2 * c[1])];
}

function setAngle(f, deg) {
    const t = ((deg % 360) + 360) % 360;
    const q = Math.round(t / 90);
    let tilt = t - q * 90;
    if (Math.abs(tilt) < 1e-9) tilt = 0;
    f.rot = ((q % 4) + 4) % 4;
    f.tilt = tilt;
}

export function moved(f, dx, dy) {
    const g = frameOf(f);
    g.c2 = [g.c2[0] + 2 * dx, g.c2[1] + 2 * dy];
    return g;
}

/** The frame turned about its pivot to the total angle `deg`. */
export function turned(f, deg) {
    const g = frameOf(f);
    const p = pivotCanvas(g);
    const c = centre(g);
    const d = turnV([c[0] - p[0], c[1] - p[1]], deg - angle(g));
    setAngle(g, deg);
    setCentre(g, [p[0] + d[0], p[1] + d[1]]);
    return g;
}

/** The frame scaled to `w x h` (before rotation) with `anchor` (a corner name or 'pivot') fixed. */
export function scaled(f, w, h, anchor) {
    const g = frameOf(f);
    const local = (fr, ww, hh) => (anchor === 'pivot' ? pivotLocal(fr) : cornerLocal(fr, anchor, ww, hh));
    const fixed = toCanvas(g, local(g, g.w, g.h));
    if (g.pivot) g.pivot = [Math.round((g.pivot[0] * w) / g.w), Math.round((g.pivot[1] * h) / g.h)];
    g.w = w;
    g.h = h;
    const off = turnV(local(g, w, h), angle(g));
    setCentre(g, [fixed[0] - off[0], fixed[1] - off[1]]);
    return g;
}

/** The nine snap points of the pivot as whole-pixel offsets from the unrotated top-left. */
export function snapPoints(f) {
    const xs = [0, f.w / 2, f.w];
    const ys = [0, f.h / 2, f.h];
    return ys.flatMap((y) => xs.map((x) => [x, y]));
}
