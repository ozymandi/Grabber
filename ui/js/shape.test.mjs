// Parity test for shape.js against the fixture generated from shape.rs (add-quad-shapes D2).
// Run: node ui/js/shape.test.mjs

import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import * as S from './shape.js';

const here = dirname(fileURLToPath(import.meta.url));
const fixture = JSON.parse(readFileSync(join(here, '../../src-tauri/tests/fixtures/shapes.json'), 'utf8'));
const TOL = fixture.tolerance;
let failures = 0;

function close(path, got, want) {
    if (want === null || got === null || want === undefined) {
        if ((want ?? null) !== (got ?? null)) fail(path, got, want);
        return;
    }
    if (Array.isArray(want)) {
        if (!Array.isArray(got) || got.length !== want.length) return fail(path, got?.length, want.length);
        want.forEach((w, i) => close(`${path}[${i}]`, got[i], w));
        return;
    }
    if (typeof want === 'object') {
        for (const k of Object.keys(want)) close(`${path}.${k}`, got[k], want[k]);
        return;
    }
    if (typeof want === 'number') {
        if (!(Math.abs(got - want) <= TOL * (1 + Math.abs(want)))) fail(path, got, want);
        return;
    }
    if (got !== want) fail(path, got, want);
}

function fail(path, got, want) {
    failures++;
    if (failures < 20) console.error(`FAIL ${path}: got ${JSON.stringify(got)}, want ${JSON.stringify(want)}`);
}

// Rust serialises a missing handle as null and a node without smooth as false.
const node = (n) => ({ p: n.p, hin: n.hin ?? null, hout: n.hout ?? null, smooth: Boolean(n.smooth) });

for (const c of fixture.cases) {
    const shape = c.shape.kind === 'path' ? { kind: 'path', nodes: c.shape.nodes.map(node) } : c.shape;
    close(`${c.name}.outline`, S.outline(shape, c.w, c.h), c.expected.outline);
    close(`${c.name}.to_path`, S.toPath(shape), c.expected.to_path.map(node));
    close(`${c.name}.empty`, S.isEmpty(shape, c.w, c.h), c.expected.empty);
}
for (const s of fixture.splits) {
    close(`split ${s.seg}`, S.split(s.nodes.map(node), s.seg, s.t), s.expected.map(node));
}

// JS-only helpers: unit square <-> image round trip through a skewed quad.
{
    const cw = [[120.5, 80.25], [910.0, 140.0], [870.0, 610.0], [60.0, 700.0]];
    for (const p of [[0, 0], [1, 1], [0.3, 0.7], [0.5, 0.5]]) {
        const back = S.imageToUnit(cw, S.unitToImage(cw, p));
        if (Math.hypot(back[0] - p[0], back[1] - p[1]) > 1e-9) fail(`unit round trip ${p}`, back, p);
    }
}

// convert keeps the kind rules.
close('convert ngon clamps', S.convert({ kind: 'rect' }, 'ngon', 40), { kind: 'ngon', n: 12 });
close('convert circle to path', S.convert({ kind: 'circle' }, 'path').nodes.length, 4);

if (failures) {
    console.error(`${failures} failure(s)`);
    process.exit(1);
}
console.log(`shape: all ${fixture.cases.length} cases and ${fixture.splits.length} splits match shape.rs`);
