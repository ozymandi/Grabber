// Quad shapes (design D1-D2 of add-quad-shapes): line-for-line port of
// src-tauri/src/shape.rs. A shape lives in the quad's unit square: (0, 0) is the first
// corner, u runs right and v down. `node ui/js/shape.test.mjs` checks this file against
// src-tauri/tests/fixtures/shapes.json, generated from shape.rs.

import * as geom from './geom.js';

export const TOLERANCE = 0.05;
export const KAPPA = 0.5522847498;
export const NGON_MIN = 3;
export const NGON_MAX = 12;
export const MIN_SHAPE_AREA = 1.0;
export const KINDS = ['rect', 'circle', 'ngon', 'path'];

export const corner = (p) => ({ p, hin: null, hout: null, smooth: false });

function circleSegments(r) {
    if (r <= TOLERANCE) return 8;
    const n = Math.ceil(Math.PI / Math.acos(1.0 - TOLERANCE / r));
    return Math.min(Math.max(Math.ceil(n / 4) * 4, 8), 4096);
}

const clampN = (n) => Math.min(Math.max(n, NGON_MIN), NGON_MAX);

function ngon(n, w, h) {
    n = clampN(n);
    const off = n % 2 === 0 ? Math.PI / n : 0.0;
    const raw = [];
    for (let k = 0; k < n; k++) {
        const a = -Math.PI / 2.0 + (2.0 * Math.PI * k) / n + off;
        raw.push([Math.cos(a), Math.sin(a)]);
    }
    let x0 = Infinity;
    let y0 = Infinity;
    let x1 = -Infinity;
    let y1 = -Infinity;
    for (const p of raw) {
        x0 = Math.min(x0, p[0]);
        y0 = Math.min(y0, p[1]);
        x1 = Math.max(x1, p[0]);
        y1 = Math.max(y1, p[1]);
    }
    return raw.map((p) => [((p[0] - x0) / (x1 - x0)) * w, ((p[1] - y0) / (y1 - y0)) * h]);
}

export const lerp = (a, b, t) => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];

function lineDist(p, a, b) {
    const dx = b[0] - a[0];
    const dy = b[1] - a[1];
    const len = Math.sqrt(dx * dx + dy * dy);
    if (len < 1e-12) return Math.sqrt((p[0] - a[0]) ** 2 + (p[1] - a[1]) ** 2);
    return Math.abs((p[0] - a[0]) * dy - (p[1] - a[1]) * dx) / len;
}

function flatten(p0, p1, p2, p3, tol, depth, out) {
    if (depth >= 16 || (lineDist(p1, p0, p3) <= tol && lineDist(p2, p0, p3) <= tol)) {
        out.push(p3);
        return;
    }
    const p01 = lerp(p0, p1, 0.5);
    const p12 = lerp(p1, p2, 0.5);
    const p23 = lerp(p2, p3, 0.5);
    const p012 = lerp(p01, p12, 0.5);
    const p123 = lerp(p12, p23, 0.5);
    const m = lerp(p012, p123, 0.5);
    flatten(p0, p01, p012, m, tol, depth + 1, out);
    flatten(m, p123, p23, p3, tol, depth + 1, out);
}

/** The four control points of path segment `i`, in unit coordinates. */
export function segment(nodes, i) {
    const a = nodes[i];
    const b = nodes[(i + 1) % nodes.length];
    return [a.p, a.hout ?? a.p, b.hin ?? b.p, b.p];
}

/** Closed outline for a `w x h` output, in output pixels, flattened to `tol`. */
export function outlineTol(shape, w, h, tol) {
    switch (shape.kind) {
        case 'circle': {
            const rx = w / 2.0;
            const ry = h / 2.0;
            const n = circleSegments((Math.max(rx, ry) * TOLERANCE) / tol);
            const out = [];
            for (let k = 0; k < n; k++) {
                const a = -Math.PI / 2.0 + (2.0 * Math.PI * k) / n;
                out.push([rx + rx * Math.cos(a), ry + ry * Math.sin(a)]);
            }
            return out;
        }
        case 'ngon':
            return ngon(shape.n, w, h);
        case 'path': {
            const nodes = shape.nodes;
            if (!nodes.length) return [];
            const sc = (p) => [p[0] * w, p[1] * h];
            const out = [sc(nodes[0].p)];
            for (let i = 0; i < nodes.length; i++) {
                const [p0, p1, p2, p3] = segment(nodes, i);
                flatten(sc(p0), sc(p1), sc(p2), sc(p3), tol, 0, out);
            }
            out.pop();
            return out;
        }
        default:
            return [[0.0, 0.0], [w, 0.0], [w, h], [0.0, h]];
    }
}

export const outline = (shape, w, h) => outlineTol(shape, w, h, TOLERANCE);

export function area(poly) {
    let s = 0.0;
    for (let i = 0; i < poly.length; i++) {
        const p = poly[i];
        const q = poly[(i + 1) % poly.length];
        s += p[0] * q[1] - q[0] * p[1];
    }
    return Math.abs(0.5 * s);
}

export function isEmpty(shape, w, h) {
    if (shape.kind === 'rect') return false;
    if (shape.kind === 'path' && shape.nodes.length < 2) return true;
    return area(outline(shape, w, h)) < MIN_SHAPE_AREA;
}

/** The shape as path anchors: Rect 4 corners, Circle 4 smooth anchors, N-gon its vertices. */
export function toPath(shape) {
    switch (shape.kind) {
        case 'circle': {
            const k = KAPPA * 0.5;
            const s = (p, hin, hout) => ({ p, hin, hout, smooth: true });
            return [
                s([0.5, 0.0], [0.5 - k, 0.0], [0.5 + k, 0.0]),
                s([1.0, 0.5], [1.0, 0.5 - k], [1.0, 0.5 + k]),
                s([0.5, 1.0], [0.5 + k, 1.0], [0.5 - k, 1.0]),
                s([0.0, 0.5], [0.0, 0.5 + k], [0.0, 0.5 - k]),
            ];
        }
        case 'ngon':
            return ngon(shape.n, 1.0, 1.0).map(corner);
        case 'path':
            return shape.nodes.map(cloneNode);
        default:
            return [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]].map(corner);
    }
}

export const cloneNode = (n) => ({ p: [...n.p], hin: n.hin ? [...n.hin] : null, hout: n.hout ? [...n.hout] : null, smooth: Boolean(n.smooth) });

/** Inserts an anchor on segment `i` at `t` without changing the curve (de Casteljau). */
export function split(nodes, i, t) {
    const n = nodes.length;
    const j = (i + 1) % n;
    const [p0, p1, p2, p3] = segment(nodes, i);
    const out = nodes.map(cloneNode);
    const straight = !nodes[i].hout && !nodes[j].hin;
    let node;
    if (straight) {
        node = corner(lerp(p0, p3, t));
    } else {
        const p01 = lerp(p0, p1, t);
        const p12 = lerp(p1, p2, t);
        const p23 = lerp(p2, p3, t);
        const p012 = lerp(p01, p12, t);
        const p123 = lerp(p12, p23, t);
        if (out[i].hout) out[i].hout = p01;
        if (out[j].hin) out[j].hin = p23;
        node = { p: lerp(p012, p123, t), hin: p012, hout: p123, smooth: true };
    }
    out.splice(i + 1, 0, node);
    return out;
}

export function cubicAt(c, t) {
    const u = 1.0 - t;
    const a = u * u * u;
    const b = 3.0 * u * u * t;
    const d = 3.0 * u * t * t;
    const e = t * t * t;
    return [a * c[0][0] + b * c[1][0] + d * c[2][0] + e * c[3][0], a * c[0][1] + b * c[1][1] + d * c[2][1] + e * c[3][1]];
}

/** The shape a quad gets when it switches to `kind` (design D2). */
export function convert(shape, kind, n) {
    if (kind === 'path') return { kind: 'path', nodes: toPath(shape) };
    if (kind === 'ngon') return { kind: 'ngon', n: clampN(n) };
    return { kind };
}

export const cloneShape = (s) => (s.kind === 'path' ? { kind: 'path', nodes: s.nodes.map(cloneNode) } : { ...s });

// --- Unit square <-> image (JS only: canvas display and hit tests) ---

/** Unit-square point -> source-image point through the quad (clockwise corners). */
export function unitToImage(cw, p) {
    const m = geom.squareToQuad(cw);
    return m ? geom.apply(m, p) : null;
}

/** Source-image point -> unit-square point, or null beyond the horizon. */
export function imageToUnit(cw, p) {
    const m = geom.squareToQuad(cw);
    const inv = m && geom.invert(m);
    if (!inv) return null;
    const w = inv[2][0] * p[0] + inv[2][1] * p[1] + inv[2][2];
    if (!(w > 0)) return null;
    return geom.apply(inv, p);
}
