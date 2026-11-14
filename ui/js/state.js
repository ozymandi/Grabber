// Editing state owned by the UI (design D3) plus derived per-quad geometry.

import * as geom from './geom.js';

/** Aspect presets shown in the Aspect segmented row. */
export const ASPECTS = {
    edges: { mode: 'edges' },
    '1:1': { mode: 'ratio', w: 1, h: 1 },
    '4:3': { mode: 'ratio', w: 4, h: 3 },
    '2:1': { mode: 'ratio', w: 2, h: 1 },
};

export const state = {
    /** { w, h, stem, rev, pixels } or null */
    source: null,
    /** [{ corners: [[x,y] x4] in click/edit order, aspectKey, custom: {w,h}, pot, dirty }]
     *  `dirty`: created or changed since it was last exported by Send or Save All. */
    quads: [],
    active: -1,
    /** Corners of the quad being created (0-3 points). */
    draft: [],
    /** Defaults for new quads; restored from settings in group 8. */
    defaults: { aspectKey: 'edges', custom: { w: 1, h: 1 }, pot: false },
    mode: 'standalone',
};

const listeners = new Set();

/** Subscribes to state changes. `what` is a hint: 'source' | 'quads' | 'active' | 'draft' | 'geometry'. */
export function onChange(fn) {
    listeners.add(fn);
    return () => listeners.delete(fn);
}

export function emit(what) {
    for (const fn of listeners) fn(what);
}

export function aspectOf(q) {
    return q.aspectKey === 'custom' ? { mode: 'ratio', w: q.custom.w, h: q.custom.h } : ASPECTS[q.aspectKey];
}

/** Derived geometry of a quad: clockwise corners, validity, output size. */
export function quadInfo(q) {
    const cw = geom.clockwise(q.corners);
    const invalid = geom.validate(cw);
    const [w, h] = geom.outputSize(cw, aspectOf(q), q.pot);
    return { cw, invalid, w, h };
}

export function activeQuad() {
    return state.quads[state.active] ?? null;
}

export function setSource(src) {
    state.source = src;
    state.quads = [];
    state.active = -1;
    state.draft = [];
    emit('source');
}

export function addQuad(corners) {
    const d = state.defaults;
    state.quads.push({
        corners: geom.clockwise(corners),
        aspectKey: d.aspectKey,
        custom: { ...d.custom },
        pot: d.pot,
        dirty: true,
    });
    state.active = state.quads.length - 1;
    emit('quads');
}

export function setActive(i) {
    if (i === state.active || i < 0 || i >= state.quads.length) return;
    state.active = i;
    emit('active');
}

export function removeQuad(i) {
    if (i < 0 || i >= state.quads.length) return;
    state.quads.splice(i, 1);
    if (state.quads.length === 0) state.active = -1;
    else if (state.active >= state.quads.length) state.active = state.quads.length - 1;
    else if (i < state.active) state.active -= 1;
    emit('quads');
}

export function moveCorner(qi, ci, p) {
    state.quads[qi].corners[ci] = p;
    state.quads[qi].dirty = true;
    emit('geometry');
}

/** Clears the unsaved mark of the quads with these 1-based numbers. */
export function markSaved(numbers) {
    for (const n of numbers) {
        const q = state.quads[n - 1];
        if (q) q.dirty = false;
    }
}

/** Quads created or changed since they were last exported by Send or Save All. */
export function unsavedCount() {
    return state.quads.filter((q) => q.dirty).length;
}
