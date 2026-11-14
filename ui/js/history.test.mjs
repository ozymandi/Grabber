// Tests for history.js (add-quad-shapes D8, spec rip-undo). Run: node ui/js/history.test.mjs

import * as H from './history.js';
import { addQuad, moveCorner, removeQuad, setActive, setShape, setSource, state } from './state.js';

let failures = 0;
function eq(name, got, want) {
    if (JSON.stringify(got) === JSON.stringify(want)) return;
    failures++;
    console.error(`FAIL ${name}: got ${JSON.stringify(got)}, want ${JSON.stringify(want)}`);
}

const sq = (x) => [[x, 0], [x + 100, 0], [x + 100, 100], [x, 100]];
const xs = () => state.quads.map((q) => q.corners[0][0]);

setSource({ w: 1000, h: 1000, stem: 't' });
addQuad(sq(0));
addQuad(sq(200));
addQuad(sq(400));
eq('three quads', xs(), [0, 200, 400]);

// Undo a removal: Q2 comes back in place and active.
setActive(1);
removeQuad(1);
eq('removed', xs(), [0, 400]);
H.undo();
eq('removal undone', [xs(), state.active], [[0, 200, 400], 1]);
H.redo();
eq('removal redone', xs(), [0, 400]);
H.undo();

// Undo a corner drag: one record per drag, however many moves.
H.record();
moveCorner(1, 0, [210, 5]);
moveCorner(1, 0, [220, 9]);
eq('dragged', state.quads[1].corners[0], [220, 9]);
H.undo();
eq('drag undone in one step', state.quads[1].corners[0], [200, 0]);

// Undo a shape change, then redo.
setShape('circle');
eq('circle', state.quads[1].shape, { kind: 'circle' });
H.undo();
eq('shape undone', state.quads[1].shape, { kind: 'rect' });
H.redo();
eq('shape redone', state.quads[1].shape, { kind: 'circle' });

// A new edit clears redo.
H.undo();
H.record();
moveCorner(0, 2, [150, 150]);
eq('redo cleared', H.redo(), false);

// Restored quads count as unsaved; untouched identical ones keep their mark.
for (const q of state.quads) q.dirty = false;
H.undo();
eq('changed quad unsaved, others kept', state.quads.map((q) => q.dirty), [true, false, false]);

// Tag merging: ratio typing is one step.
H.record('ratio');
state.quads[0].aspectKey = 'custom';
H.record('ratio');
state.quads[0].custom = { w: 3, h: 1 };
H.undo();
eq('ratio typing undone at once', [state.quads[0].aspectKey, state.quads[0].custom], ['edges', { w: 1, h: 1 }]);

// The cap.
for (let i = 0; i < 150; i++) {
    H.record();
    moveCorner(0, 0, [i, 0]);
}
let n = 0;
while (H.undo()) n++;
eq('history keeps 100 steps', n, H.LIMIT);

// A new image clears the history.
setSource({ w: 10, h: 10, stem: 'u' });
eq('new image clears', [H.canUndo(), H.canRedo(), H.undo()], [false, false, false]);

if (failures) {
    console.error(`${failures} failure(s)`);
    process.exit(1);
}
console.log('history: all tests passed');
