// Line-for-line port of src-tauri/src/geom.rs, used only for the preview and the UI.
// Export geometry is always recomputed in Rust. tests: `node ui/js/geom.test.mjs`
// checks this file against src-tauri/tests/fixtures/quads.json (design D5).

export const MIN_AREA = 4.0;
export const MIN_ANGLE_DEG = 2.0;
export const MAX_SIDE = 16384;

/** Unit square -> quad: (0,0)->c[0], (1,0)->c[1], (1,1)->c[2], (0,1)->c[3]. Row-major 3x3 or null. */
export function squareToQuad(c) {
    const [[x0, y0], [x1, y1], [x2, y2], [x3, y3]] = c;
    const sx = x0 - x1 + x2 - x3;
    const sy = y0 - y1 + y2 - y3;
    const dx1 = x1 - x2;
    const dx2 = x3 - x2;
    const dy1 = y1 - y2;
    const dy2 = y3 - y2;
    const den = dx1 * dy2 - dx2 * dy1;
    if (den === 0 || !Number.isFinite(den)) return null;
    const g = (sx * dy2 - dx2 * sy) / den;
    const h = (dx1 * sy - sx * dy1) / den;
    const m = [
        [x1 - x0 + g * x1, x3 - x0 + h * x3, x0],
        [y1 - y0 + g * y1, y3 - y0 + h * y3, y0],
        [g, h, 1.0],
    ];
    return m.flat().every(Number.isFinite) ? m : null;
}

/** Output pixel coordinates (0..w, 0..h) -> source pixels. */
export function outputToSource(c, w, h) {
    const m = squareToQuad(c);
    if (!m) return null;
    const sx = 1.0 / Math.max(w, 1);
    const sy = 1.0 / Math.max(h, 1);
    return [
        [m[0][0] * sx, m[0][1] * sy, m[0][2]],
        [m[1][0] * sx, m[1][1] * sy, m[1][2]],
        [m[2][0] * sx, m[2][1] * sy, m[2][2]],
    ];
}

export function apply(m, p) {
    const x = m[0][0] * p[0] + m[0][1] * p[1] + m[0][2];
    const y = m[1][0] * p[0] + m[1][1] * p[1] + m[1][2];
    const w = m[2][0] * p[0] + m[2][1] * p[1] + m[2][2];
    return Math.abs(w) < 1e-12 ? null : [x / w, y / w];
}

export function invert(m) {
    const [[a, b, c], [d, e, f], [g, h, i]] = m;
    const co00 = e * i - f * h;
    const co01 = f * g - d * i;
    const co02 = d * h - e * g;
    const det = a * co00 + b * co01 + c * co02;
    if (Math.abs(det) < 1e-300 || !Number.isFinite(det)) return null;
    const inv = 1.0 / det;
    return [
        [co00 * inv, (c * h - b * i) * inv, (b * f - c * e) * inv],
        [co01 * inv, (a * i - c * g) * inv, (c * d - a * f) * inv],
        [co02 * inv, (b * g - a * h) * inv, (a * e - b * d) * inv],
    ];
}

export function signedArea(c) {
    let s = 0.0;
    for (let i = 0; i < 4; i++) {
        const p = c[i];
        const q = c[(i + 1) % 4];
        s += p[0] * q[1] - q[0] * p[1];
    }
    return 0.5 * s;
}

/** Clockwise screen order, keeping the first corner first. */
export function clockwise(c) {
    return signedArea(c) < 0.0 ? [c[0], c[3], c[2], c[1]] : c.slice();
}

/** null when valid, otherwise 'not_finite' | 'not_convex' | 'too_small' | 'collinear'. */
export function validate(c) {
    if (c.flat().some((v) => !Number.isFinite(v))) return 'not_finite';
    for (let i = 0; i < 4; i++) {
        const a = c[i];
        const b = c[(i + 1) % 4];
        const d = c[(i + 2) % 4];
        const cross = (b[0] - a[0]) * (d[1] - b[1]) - (b[1] - a[1]) * (d[0] - b[0]);
        if (cross <= 0.0) return cross === 0.0 ? 'collinear' : 'not_convex';
    }
    if (signedArea(c) < MIN_AREA) return 'too_small';
    const lo = (MIN_ANGLE_DEG * Math.PI) / 180;
    const hi = ((180.0 - MIN_ANGLE_DEG) * Math.PI) / 180;
    for (let i = 0; i < 4; i++) {
        const p = c[(i + 3) % 4];
        const v = c[i];
        const n = c[(i + 1) % 4];
        const a = [p[0] - v[0], p[1] - v[1]];
        const b = [n[0] - v[0], n[1] - v[1]];
        const cross = a[0] * b[1] - a[1] * b[0];
        const dot = a[0] * b[0] + a[1] * b[1];
        const ang = Math.atan2(Math.abs(cross), dot);
        if (!(ang >= lo && ang <= hi)) return 'collinear';
    }
    return null;
}

export function edgeSize(c) {
    const len = (a, b) => Math.sqrt((b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2);
    const top = len(c[0], c[1]);
    const bottom = len(c[3], c[2]);
    const left = len(c[0], c[3]);
    const right = len(c[1], c[2]);
    return [(top + bottom) * 0.5, (left + right) * 0.5];
}

export function nearestPot(n) {
    if (n <= 1) return 1;
    if ((n & (n - 1)) === 0) return n;
    let hi = 1;
    while (hi < n) hi *= 2;
    const lo = hi / 2;
    return n - lo < hi - n ? lo : hi;
}

// Rust's f64::round rounds half away from zero; Math.round rounds half up. Sides are
// non-negative, where the two agree.
function toSide(v) {
    if (!Number.isFinite(v)) return 1;
    return Math.min(Math.max(Math.round(v), 1), MAX_SIDE);
}

/** Output size [w, h]. `aspect` is {mode:'edges'} or {mode:'ratio', w, h}. */
export function outputSize(c, aspect, pot) {
    const [we, he] = edgeSize(c);
    const w = toSide(we);
    let h;
    if (aspect.mode === 'ratio' && aspect.w > 0 && aspect.h > 0) h = toSide((w * aspect.h) / aspect.w);
    else h = toSide(he);
    return pot ? [Math.min(nearestPot(w), MAX_SIDE), Math.min(nearestPot(h), MAX_SIDE)] : [w, h];
}
