// Tests for marquee.js against the scenarios of add-atlas-marquee (spec atlas-tiles).
// Run: node ui/js/atlas/marquee.test.mjs

import * as F from './frame.js';
import { boxFrom, touches } from './marquee.js';

let failures = 0;

function eq(name, got, want) {
    if (JSON.stringify(got) === JSON.stringify(want)) return;
    failures++;
    console.error(`FAIL ${name}: got ${JSON.stringify(got)}, want ${JSON.stringify(want)}`);
}

/** An unrotated tile whose outline is x0..x1, y0..y1. */
const tile = (x0, y0, x1, y1) => ({ c2: [x0 + x1, y0 + y1], w: x1 - x0, h: y1 - y0, rot: 0, tilt: 0, pivot: null });
const hit = (t, a, b) => touches(F.corners(t), boxFrom(a, b));

// Unrotated tile at 100..300 x 100..200.
{
    const t = tile(100, 100, 300, 200);
    eq('frame inside the tile', hit(t, [150, 120], [160, 130]), true);
    eq('frame around the tile', hit(t, [0, 0], [400, 400]), true);
    eq('frame over one corner', hit(t, [280, 180], [350, 250]), true);
    eq('frame flush with the right edge', hit(t, [300, 120], [350, 130]), true);
    eq('frame 1 px right of the tile', hit(t, [301, 120], [350, 130]), false);
    eq('frame 1 px above the tile', hit(t, [150, 50], [160, 99]), false);
    eq('right-to-left, bottom-to-top', hit(t, [350, 250], [280, 180]), true);
}

// "Rotated outline, not the bounding box": a 200 x 100 tile turned 45° about (500, 500).
{
    const t = { c2: [1000, 1000], w: 200, h: 100, rot: 0, tilt: 45, pivot: null };
    const r = F.rect(t);
    // The bounding box's top-left corner region is empty: the outline does not reach it.
    eq('bounding-box corner, off the tile', hit(t, [r.x, r.y], [r.x + 20, r.y + 20]), false);
    eq('bounding-box corner region, from the other side', hit(t, [r.x + 20, r.y + 20], [r.x, r.y]), false);
    // The tip of the tile: its right-most corner.
    const tip = F.corners(t).reduce((a, p) => (p[0] > a[0] ? p : a));
    eq('frame over the tip', hit(t, [tip[0] - 5, tip[1] - 5], [tip[0] + 20, tip[1] + 20]), true);
    eq('frame 2 px past the tip', hit(t, [tip[0] + 2, tip[1] - 5], [tip[0] + 20, tip[1] + 5]), false);
    eq('frame across the centre', hit(t, [495, 400], [505, 600]), true);
}

// A quarter-turned tile behaves as its turned rectangle.
{
    const t = { c2: [400, 400], w: 200, h: 100, rot: 1, tilt: 0, pivot: null };
    // Turned: 100 wide (150..250) and 200 tall (100..300).
    eq('quarter turn: inside the turned outline', hit(t, [160, 280], [170, 290]), true);
    eq('quarter turn: where the unturned tile was', hit(t, [260, 160], [290, 170]), false);
}

if (failures) {
    console.error(`${failures} failure(s)`);
    process.exit(1);
}
console.log('marquee: all tests passed');
