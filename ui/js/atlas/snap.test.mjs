// Tests for snap.js against the scenarios of add-atlas-snapping (spec atlas-tiles).
// Run: node ui/js/atlas/snap.test.mjs

import * as F from './frame.js';
import { boxOf, gridStep, snapMove, targets } from './snap.js';

let failures = 0;

function eq(name, got, want) {
    if (JSON.stringify(got) === JSON.stringify(want)) return;
    failures++;
    console.error(`FAIL ${name}: got ${JSON.stringify(got)}, want ${JSON.stringify(want)}`);
}

/** An unrotated tile whose bounding box is x0..x1, y0..y1. */
const tile = (id, x0, y0, x1, y1) => ({ id, c2: [x0 + x1, y0 + y1], w: x1 - x0, h: y1 - y0, rot: 0, tilt: 0, pivot: null });

/** Sector 1001 spans x 0..1024, y -1024..0 for a 1024 workspace. */
const workspace = (tiles, padding = 8) => ({ sector_size: 1024, padding, sectors: [{ u: 0, v: 0 }], tiles });

/** Moves the tiles in `ids` so the box's left edge would land at `left` and returns the snapped left edge. */
function dropLeft(m, ids, left, k = 1, dpr = 1) {
    const sel = new Set(ids);
    const box = boxOf(m.tiles.filter((t) => sel.has(t.id)));
    const r = snapMove(box, left - box.x0, 0, targets(m, sel), m.sector_size, k, dpr);
    return box.x0 + r.dx;
}

// Rows at y -608..-508 lie on 16 px grid lines, so only x is in question.
const moving = () => tile('a', 400, -608, 500, -508);

// gridStep / spec atlas-guides "Grid on a 2048 sector" and "Zoomed out".
eq('grid 2048 at 100 %', gridStep(2048, 1, 1), 32);
eq('grid 2048 zoomed out', gridStep(2048, 3 / 32, 1), 256);
eq('grid with dpr 2', gridStep(1024, 0.5, 2), 128);

// Targets.
{
    const m = workspace([moving(), tile('b', 200, -608, 300, -508)]);
    const tg = targets(m, new Set(['a']));
    eq('sector and tile edges on x, padding 8', tg.x.edges.map((e) => e.value), [0, 8, 1024, 1016, 200, 192, 300, 308]);
    eq('padded targets keep their edge', tg.x.edges.map((e) => e.at), [0, 0, 1024, 1024, 200, 200, 300, 300]);
    eq('selected tile is no target', tg.x.edges.some((e) => e.value === 400 || e.value === 500), false);
    const flat = targets(workspace([moving()], 0), new Set(['a']));
    eq('padding 0 gives flush only', flat.x.edges.map((e) => e.value), [0, 1024]);
}

// "Flush to a sector border".
eq('flush to a sector border', dropLeft(workspace([moving()]), ['a'], 3), 0);

// "At the padding from a sector border".
eq('at the padding from a sector border', dropLeft(workspace([moving()]), ['a'], 10), 8);

// "Beside another tile at the padding".
{
    const m = workspace([moving(), tile('b', 200, -608, 300, -508)]);
    const sel = new Set(['a']);
    const box = boxOf([m.tiles[0]]);
    const r = snapMove(box, 310 - box.x0, 0, targets(m, sel), 1024, 1, 1);
    eq('beside another tile at the padding', box.x0 + r.dx, 308);
    // Spec "Line beside a tile": the line runs along the target edge at 300, not at 308.
    eq('snap line along the tile edge', r.lines.find((l) => l.axis === 'x'), { axis: 'x', value: 300, span: [-608, -508] });
    const inner = snapMove(box, 10 - box.x0, 0, targets(m, sel), 1024, 1, 1);
    eq('padding inside a sector: line on the border', inner.lines.find((l) => l.axis === 'x').value, 0);
}

// "Tile edge wins over the grid": grid line at 304 is one pixel away, the tile edge three.
eq('tile edge wins over the grid', dropLeft(workspace([moving(), tile('b', 200, -608, 300, -508)], 0), ['a'], 303), 300);

// Grid alone attracts when no edge is in reach.
eq('grid line attracts', dropLeft(workspace([moving()]), ['a'], 403), 400);
eq('grid line 3 px off', dropLeft(workspace([moving()]), ['a'], 419), 416);

// "Hidden minor lines do not attract": minor lines 3 screen px apart, edge 1 screen px off one.
{
    const k = 3 / 16;
    // x 213 is 5 canvas px (under 1 screen px) from the hidden line at 208 and 43 from the major line at 256.
    eq('hidden minor lines do not attract', dropLeft(workspace([moving()]), ['a'], 213, k), 213);
    eq('major line still attracts', dropLeft(workspace([moving()]), ['a'], 250, k), 256);
}

// "Threshold follows the zoom": tray tiles, padding 0.
{
    const tray = () => workspace([tile('a', 2200, 100, 2300, 200), tile('b', 1900, 100, 2000, 200)], 0);
    eq('threshold at 50 %', dropLeft(tray(), ['a'], 2014, 0.5), 2000);
    eq('threshold at 200 %', dropLeft(tray(), ['a'], 2014, 2), 2014);
    eq('threshold with dpr 2 at 100 %', dropLeft(tray(), ['a'], 2014, 2, 2), 2014);
}

// "Axes snap on their own".
{
    const m = workspace([tile('a', 2200, 100, 2300, 200), tile('b', 1900, 100, 2000, 200)], 0);
    const box = boxOf([m.tiles[0]]);
    const r = snapMove(box, 2003 - box.x0, 500, targets(m, new Set(['a'])), 1024, 1, 1);
    eq('x snaps', box.x0 + r.dx, 2000);
    eq('y follows', r.dy, 500);
    eq('one line, on x', r.lines.map((l) => l.axis), ['x']);
}

// "Several tiles move as one box": the union's left edge snaps, the selection is no target.
{
    const m = workspace([tile('a', 400, -608, 500, -508), tile('c', 520, -608, 620, -508), tile('d', 410, -400, 480, -300)]);
    eq('several tiles as one box', dropLeft(m, ['a', 'c', 'd'], 5), 8);
    const tg = targets(m, new Set(['a', 'c', 'd']));
    eq('no selected edge is a target', tg.x.edges.length, 4);
}

// "Turned tile": the bounding box of a 45° tile lands flush.
{
    const t = { id: 'a', c2: [800, -1200], w: 200, h: 100, rot: 0, tilt: 45, pivot: null };
    const m = workspace([t]);
    const box = boxOf([t]);
    const r = snapMove(box, 3 - box.x0, 0, targets(m, new Set(['a'])), 1024, 1, 1);
    eq('turned tile box flush', F.rect(F.moved(t, r.dx, r.dy)).x, 0);
}

// Results are whole pixels.
{
    const m = workspace([moving(), tile('b', 200, -608, 300, -508)]);
    const box = boxOf([m.tiles[0]]);
    const r = snapMove(box, -91, -3, targets(m, new Set(['a'])), 1024, 0.37, 1.25);
    eq('integer offsets', [Number.isInteger(r.dx), Number.isInteger(r.dy)], [true, true]);
}

if (failures) {
    console.error(`${failures} failure(s)`);
    process.exit(1);
}
console.log('snap: all tests passed');
