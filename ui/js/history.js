// Undo and redo of quad edits in RIP (design D8 of add-quad-shapes): snapshots of the quad
// list and the active index, taken before each change. The history lives in memory and is
// cleared when a new source image is loaded. Test: `node ui/js/history.test.mjs`.

import { emit, state } from './state.js';

export const LIMIT = 100;

let past = [];
let future = [];
/** Tag of the last record; a record with the same tag is merged into it (ratio typing). */
let lastTag = null;

/** Quads without their unsaved mark, as JSON. */
const strip = (quads) => quads.map(({ dirty, ...q }) => q);
const snapshot = () => ({ quads: JSON.stringify(strip(state.quads)), active: state.active });

/**
 * Records the state before a change. Consecutive records with the same non-null `tag`
 * count as one step, so typing a ratio digit by digit is undone at once.
 */
export function record(tag = null) {
    if (tag !== null && tag === lastTag) return;
    lastTag = tag;
    past.push(snapshot());
    if (past.length > LIMIT) past.shift();
    future = [];
}

/** Ends tag merging, for example when the ratio field loses focus. */
export function endTag() {
    lastTag = null;
}

function restore(snap) {
    // A restored quad keeps its unsaved mark only when an identical quad exists now;
    // anything undo or redo changed counts as unsaved (spec "Undo after Send").
    const now = new Map(state.quads.map((q) => [JSON.stringify(strip([q])[0]), q.dirty]));
    state.quads = JSON.parse(snap.quads).map((q) => {
        const key = JSON.stringify(q);
        return { ...q, dirty: now.has(key) ? now.get(key) : true };
    });
    state.active = state.quads.length ? Math.min(Math.max(snap.active, 0), state.quads.length - 1) : -1;
    state.draft = [];
    lastTag = null;
    emit('quads');
}

export function undo() {
    if (!past.length) return false;
    future.push(snapshot());
    restore(past.pop());
    return true;
}

export function redo() {
    if (!future.length) return false;
    past.push(snapshot());
    restore(future.pop());
    return true;
}

export function clear() {
    past = [];
    future = [];
    lastTag = null;
}

export const canUndo = () => past.length > 0;
export const canRedo = () => future.length > 0;
