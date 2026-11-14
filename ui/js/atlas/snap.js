// Snapping while moving tiles in ATLAS (design D1, D2, D5 of add-atlas-snapping). Pure: no DOM,
// so `snap.test.mjs` runs it under node. All targets, box edges and grid lines are whole canvas
// pixels, so every correction is an integer and a snapped edge lands exactly on its target.

import * as F from './frame.js';

export const SNAP = 8; // snap distance, CSS px
export const MINOR_MIN = 6; // minor grid lines closer than this are hidden, CSS px

/** Grid step in canvas pixels at scale `k` (device px per canvas px): S/64 while minor lines show, else S/8. */
export function gridStep(size, k, dpr) {
    return (size / 64) * k >= MINOR_MIN * dpr ? size / 64 : size / 8;
}

/** Bounding box of tile frames as { x0, y0, x1, y1 }. */
export function boxOf(tiles) {
    const rs = tiles.map((t) => F.rect(t));
    return {
        x0: Math.min(...rs.map((r) => r.x)),
        y0: Math.min(...rs.map((r) => r.y)),
        x1: Math.max(...rs.map((r) => r.x + r.w)),
        y1: Math.max(...rs.map((r) => r.y + r.h)),
    };
}

/**
 * Snap targets for a move of the tiles in `selected` (a Set of ids). Per axis, `edges` holds
 * { value, span } (span = the target's extent on the other axis) and `sectors` holds each
 * sector's { lo, hi, span } on that axis, for grid lines.
 */
export function targets(m, selected) {
    const S = m.sector_size;
    const p = m.padding;
    const x = { edges: [], sectors: [] };
    const y = { edges: [], sectors: [] };
    // Each edge attracts flush and, with padding, at `inward` * p from it; `at` is the edge
    // itself, where the snap line is drawn.
    const add = (axis, at, inward, span) => {
        axis.edges.push({ value: at, at, span });
        if (p > 0) axis.edges.push({ value: at + inward * p, at, span });
    };
    for (const s of m.sectors) {
        const r = { x0: s.u * S, y0: -(s.v + 1) * S, x1: (s.u + 1) * S, y1: -s.v * S };
        add(x, r.x0, 1, [r.y0, r.y1]);
        add(x, r.x1, -1, [r.y0, r.y1]);
        add(y, r.y0, 1, [r.x0, r.x1]);
        add(y, r.y1, -1, [r.x0, r.x1]);
        x.sectors.push({ lo: r.x0, hi: r.x1, span: [r.y0, r.y1] });
        y.sectors.push({ lo: r.y0, hi: r.y1, span: [r.x0, r.x1] });
    }
    for (const t of m.tiles) {
        if (selected.has(t.id)) continue;
        const r = F.rect(t);
        const [x0, y0, x1, y1] = [r.x, r.y, r.x + r.w, r.y + r.h];
        add(x, x0, -1, [y0, y1]);
        add(x, x1, 1, [y0, y1]);
        add(y, y0, -1, [x0, x1]);
        add(y, y1, 1, [x0, x1]);
    }
    return { x, y };
}

/** Distance between two ranges on the other axis; 0 when they overlap. */
const gap = (a, b) => Math.max(0, a[0] - b[1], b[0] - a[1]);

/**
 * Snaps one axis of the moving box [lo, hi] (its extent on the other axis is `cross`).
 * `limit` is the snap distance in canvas pixels and `step` the grid step. Returns
 * { corr, at, span } or null, where `at` is the target edge for the snap line. Sector
 * borders and tile edges win over grid lines; otherwise the nearest wins, and among
 * equals the target closest on the other axis.
 */
export function snapAxis(lo, hi, cross, axis, step, limit) {
    let best = null;
    const consider = (value, at, span, edge) => {
        const corr = value - edge;
        const a = Math.abs(corr);
        if (a > limit) return;
        const g = gap(span, cross);
        if (!best || a < best.a || (a === best.a && g < best.g)) best = { a, g, corr, at, span };
    };
    for (const t of axis.edges) {
        consider(t.value, t.at, t.span, lo);
        consider(t.value, t.at, t.span, hi);
    }
    if (!best) {
        for (const edge of [lo, hi]) {
            const value = Math.round(edge / step) * step;
            for (const s of axis.sectors) if (s.lo < value && value < s.hi) consider(value, value, s.span, edge);
        }
    }
    return best && { corr: best.corr, at: best.at, span: best.span };
}

/**
 * Snaps a move of `box` by the raw offset (rx, ry). `k` is the view scale in device px per
 * canvas px. Returns the snapped { dx, dy } and the snap lines { axis, value, span }.
 */
export function snapMove(box, rx, ry, tg, size, k, dpr) {
    const limit = (SNAP * dpr) / k;
    const step = gridStep(size, k, dpr);
    const b = { x0: box.x0 + rx, y0: box.y0 + ry, x1: box.x1 + rx, y1: box.y1 + ry };
    const sx = snapAxis(b.x0, b.x1, [b.y0, b.y1], tg.x, step, limit);
    const sy = snapAxis(b.y0, b.y1, [b.x0, b.x1], tg.y, step, limit);
    const lines = [];
    if (sx) lines.push({ axis: 'x', value: sx.at, span: sx.span });
    if (sy) lines.push({ axis: 'y', value: sy.at, span: sy.span });
    return { dx: rx + (sx ? sx.corr : 0), dy: ry + (sy ? sy.corr : 0), lines };
}
