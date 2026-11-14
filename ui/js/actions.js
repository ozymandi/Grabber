// Export actions (spec texture-export): SEND ACTIVE / Enter, SAVE ALL / Ctrl+S, COPY, and
// closing. Rust renders and writes; this module builds the request and reports the result.

import { invalidReason } from './panel-quads.js';
import { invoke } from './source.js';
import { activeQuad, aspectOf, markSaved, quadInfo, state } from './state.js';
import { setStatus } from './status.js';

const toReq = (q, i) => ({ corners: q.corners, aspect: aspectOf(q), pot: q.pot, number: i + 1 });

let busy = false;

function failureText(f) {
    return `${f.target}${f.path ? ` (${f.path})` : ''}: ${f.error}`;
}

/** Atlas part of a status line, or null when the Atlas output is off. */
function atlasPart(r) {
    const a = r.atlas;
    if (!a) return null;
    if (a.error) return `Atlas: ${a.error}`;
    const tray = a.tray ? ` (${a.tray} beside the grid, not exported)` : '';
    return `atlas ${a.name || 'Untitled'} +${a.added}${tray}`;
}

/** Shared tail of a status line: folder count or failures. */
function folderPart(r) {
    if (r.folders === 0) return 'no folder enabled';
    return `${r.written.length} file${r.written.length === 1 ? '' : 's'} in ${r.folders} folder${r.folders === 1 ? '' : 's'}`;
}

async function run(kind, quads) {
    if (busy) return null;
    busy = true;
    try {
        return await invoke('export', { req: { kind, quads } });
    } catch (e) {
        setStatus(String(e).replace(/^Error: /, ''), 'error');
        return null;
    } finally {
        busy = false;
    }
}

/** Checks there is a valid active quad; otherwise explains why nothing was written. */
function activeOrExplain() {
    const q = activeQuad();
    if (!state.source) return setStatus('Open, paste or drop an image first', 'error'), null;
    if (!q) return setStatus('Nothing to send: mark a quad first', 'error'), null;
    const info = quadInfo(q);
    if (info.invalid) {
        setStatus(`Nothing to send: Q${state.active + 1} is invalid (${invalidReason(info.invalid).toLowerCase()})`, 'error');
        return null;
    }
    return q;
}

export async function sendActive() {
    const q = activeOrExplain();
    if (!q) return;
    const n = state.active + 1;
    const r = await run('send', [toReq(q, state.active)]);
    if (!r) return;
    markSaved(r.saved);
    if (r.exit_after) {
        await invoke('quit');
        return;
    }
    const size = r.size ? ` ${r.size[0]} × ${r.size[1]}` : '';
    const atlas = atlasPart(r);
    if (r.failures.length || r.atlas?.error) {
        const problems = r.failures.map(failureText);
        if (r.atlas?.error) problems.push(atlas);
        setStatus(`Q${n}${size} sent with problems. ${problems.join('; ')}`, 'error');
    } else {
        const parts = ['clipboard'];
        if (r.output) parts.push('ShareX');
        setStatus(`Sent Q${n}${size} to ${parts.join(', ')}; ${folderPart(r)}${atlas ? `; ${atlas}` : ''}`, 'success');
    }
}

export async function copyActive() {
    const q = activeOrExplain();
    if (!q) return;
    const r = await run('copy', [toReq(q, state.active)]);
    if (!r) return;
    if (r.failures.length) setStatus(r.failures.map(failureText).join('; '), 'error');
    else setStatus(`Copied Q${state.active + 1} ${r.size[0]} × ${r.size[1]} to the clipboard`, 'success');
}

export async function saveAll() {
    if (!state.source || state.quads.length === 0) {
        setStatus('Nothing to save: mark a quad first', 'error');
        return;
    }
    const r = await run('all', state.quads.map(toReq));
    if (!r) return;
    markSaved(r.saved);
    const skipped = r.skipped.map((s) => `Q${s.number} skipped (${invalidReason(s.reason).toLowerCase()})`);
    const atlas = atlasPart(r);
    if (atlas) skipped.unshift(atlas);
    if (r.folders === 0 && r.atlas && !r.atlas.error) {
        const saved = state.quads.length - r.skipped.length;
        setStatus([`Saved ${saved} quad${saved === 1 ? '' : 's'}`, ...skipped].join('; '), r.skipped.length ? 'error' : 'success');
    } else if (r.atlas?.error) {
        setStatus([`Saved ${r.written.length} file(s) with problems`, ...r.failures.map(failureText), ...skipped].join('; '), 'error');
    } else if (r.folders === 0) {
        setStatus(['Nothing saved: no output folder is enabled', ...skipped].join('; '), 'error');
    } else if (r.failures.length) {
        setStatus([`Saved ${r.written.length} file(s) with problems`, ...r.failures.map(failureText), ...skipped].join('; '), 'error');
    } else {
        const saved = state.quads.length - r.skipped.length;
        setStatus([`Saved ${saved} quad${saved === 1 ? '' : 's'}: ${folderPart(r)}`, ...skipped].join('; '), skipped.length ? 'error' : 'success');
    }
}

export const quit = () => invoke('quit');
