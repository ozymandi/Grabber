// Parity test for geom.js against the fixture generated from geom.rs (design D5).
// Run: node ui/js/geom.test.mjs

import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import * as geom from './geom.js';

const here = dirname(fileURLToPath(import.meta.url));
const fixture = JSON.parse(readFileSync(join(here, '../../src-tauri/tests/fixtures/quads.json'), 'utf8'));
const TOL = fixture.tolerance;

let failures = 0;

function close(path, got, want) {
    if (want === null || got === null) {
        if (want !== got) fail(path, got, want);
        return;
    }
    if (Array.isArray(want)) {
        if (!Array.isArray(got) || got.length !== want.length) return fail(path, got, want);
        want.forEach((w, i) => close(`${path}[${i}]`, got[i], w));
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
    console.error(`FAIL ${path}: got ${JSON.stringify(got)}, want ${JSON.stringify(want)}`);
}

for (const c of fixture.cases) {
    const e = c.expected;
    const cw = geom.clockwise(c.corners);
    const v = geom.validate(cw);
    close(`${c.name}.clockwise`, cw, e.clockwise);
    close(`${c.name}.valid`, v === null ? true : v, e.valid);
    const [w, h] = geom.outputSize(cw, c.aspect, c.pot);
    close(`${c.name}.size`, [w, h], e.size);
    close(`${c.name}.square_to_quad`, geom.squareToQuad(cw), e.square_to_quad);
    close(`${c.name}.output_to_source`, geom.outputToSource(cw, w, h), e.output_to_source);
}

if (failures) {
    console.error(`${failures} mismatch(es) across ${fixture.cases.length} cases`);
    process.exit(1);
}
console.log(`geom.js matches geom.rs on ${fixture.cases.length} cases (tolerance ${TOL})`);
