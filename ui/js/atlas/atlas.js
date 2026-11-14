// ATLAS mode (OpenSpec change add-texture-atlas): panels, workspace commands, change polling
// and keys. Rust owns the workspace; every command returns the new document (design D3).

import { setStatus } from '../status.js';
import { AtlasCanvas, sectorRect } from './canvas.js';

const el = (id) => document.getElementById(id);
const invoke = (cmd, args) => window.__TAURI__.core.invoke(cmd, args);
const errText = (e) => String(e).replace(/^Error: /, '');

let canvas = null;
let doc = null;
let visible = false;
let scope = 'sector';
let nameTouched = false;
let polling = null;
let busy = false;

const fileName = (p) => (p ? p.split(/[\\/]/).pop() : null);

function tilesOn(sector) {
    const m = doc.manifest;
    const r = sectorRect(sector, m.sector_size);
    return m.tiles.some((t) => t.x < r.x + r.w && r.x < t.x + t.w && t.y < r.y + r.h && r.y < t.y + t.h);
}

function udimOf(sector) {
    const m = doc.manifest;
    const umin = Math.min(...m.sectors.map((s) => s.u));
    const vmin = Math.min(...m.sectors.map((s) => s.v));
    return 1001 + (sector.u - umin) + 10 * (sector.v - vmin);
}

function render() {
    if (!doc) return;
    const m = doc.manifest;
    el('atlasName').value = m.name;
    el('atlasName').title = doc.file ?? 'Not saved yet';
    el('atlasUnsaved').hidden = !doc.unsaved;
    const locked = m.tiles.length > 0;
    for (const b of el('atlasSizeSeg').querySelectorAll('.seg')) {
        b.classList.toggle('is-active', Number(b.dataset.size) === m.sector_size);
        b.disabled = locked && Number(b.dataset.size) !== m.sector_size;
        b.title = locked ? 'The sector size can only change while the workspace has no tiles' : `Sectors of ${b.dataset.size} px`;
    }
    if (document.activeElement !== el('atlasPadding')) el('atlasPadding').value = String(m.padding);

    const sel = canvas.selSectors;
    const removeBtn = el('atlasRemoveSectorBtn');
    if (sel.length === 1) {
        el('atlasSectorName').textContent = String(udimOf(sel[0]));
        const blocked = tilesOn(sel[0]) ? 'Move or delete the tiles on this sector to remove it' : m.sectors.length === 1 ? 'A workspace keeps at least one sector' : '';
        removeBtn.disabled = Boolean(blocked);
        el('atlasSectorHint').textContent = blocked || 'Use + on its sides to add a neighbour';
    } else {
        el('atlasSectorName').textContent = sel.length > 1 ? `${sel.length} selected` : '';
        removeBtn.disabled = true;
        el('atlasSectorHint').textContent = sel.length > 1 ? 'Select one sector to add or remove neighbours' : 'Click a sector to select it';
    }
    const n = sel.length;
    el('atlasExportCount').textContent = n === 0 ? 'active sector' : `${n} sector${n === 1 ? '' : 's'}`;
    for (const b of el('atlasScopeSeg').querySelectorAll('.seg')) b.classList.toggle('is-active', b.dataset.scope === scope);
    if (!nameTouched) el('atlasExportName').value = m.name;
}

function setDoc(d) {
    if (doc && d.id !== doc.id) nameTouched = false;
    doc = d;
    canvas.setDoc(d);
    render();
}

/** Runs a workspace command that returns a document; failures go to the status row. */
async function run(fn, ok) {
    if (busy) return null;
    busy = true;
    try {
        const d = await fn();
        if (d) setDoc(d);
        if (ok) setStatus(typeof ok === 'function' ? ok(d) : ok, 'success');
        return d;
    } catch (e) {
        setStatus(errText(e), 'error');
        return null;
    } finally {
        busy = false;
    }
}

async function edit(e, opts = {}) {
    try {
        const r = await invoke('atlas_edit', { edit: e });
        setDoc(r.doc);
        if (e.op === 'delete') setStatus(`Removed ${r.removed} tile${r.removed === 1 ? '' : 's'}`, 'success');
        return r;
    } catch (err) {
        if (!opts.quiet) setStatus(errText(err), 'error');
        // Re-sync: the canvas may show an optimistic position.
        await reload();
        return null;
    }
}

async function reload() {
    try {
        setDoc(await invoke('atlas_get'));
    } catch (e) {
        setStatus(errText(e), 'error');
    }
}

const filters = [{ name: 'ripper atlas', extensions: ['ripper'] }];

async function saveAs() {
    const path = await window.__TAURI__.dialog.save({ title: 'Save the atlas workspace', filters, defaultPath: `${doc.manifest.name}.ripper` });
    if (typeof path !== 'string') return;
    await run(() => invoke('atlas_save_as', { path }), (d) => `Saved ${fileName(d.file)}`);
}

async function save() {
    if (!doc) return;
    const saved = await run(() => invoke('atlas_save'), (d) => (d ? `Saved ${fileName(d.file)}` : ''));
    if (saved === null && !doc.file) await saveAs();
}

async function openFile() {
    const path = await window.__TAURI__.dialog.open({ title: 'Open an atlas workspace', multiple: false, directory: false, filters });
    if (typeof path !== 'string') return;
    await run(() => invoke('atlas_open', { path }), (d) => `Opened ${fileName(d.file)}${d.unsaved ? ' with unsaved changes restored' : ''}`);
}

async function newWorkspace() {
    await run(() => invoke('atlas_new'), 'New workspace');
}

async function exportSectors() {
    const base = el('atlasExportName').value.trim();
    const dir = await window.__TAURI__.dialog.open({ title: 'Choose the export folder', directory: true, multiple: false });
    if (typeof dir !== 'string') return;
    try {
        const files = await invoke('atlas_export', { scope, sectors: canvas.selSectors, dir, base });
        const names = files.map(fileName);
        const list = names.length <= 2 ? names.join(', ') : `${names[0]} … ${names[names.length - 1]}`;
        setStatus(`Exported ${names.length} file${names.length === 1 ? '' : 's'}: ${list}`, 'success');
    } catch (e) {
        setStatus(errText(e), 'error');
    }
}

function selectedTiles() {
    return [...canvas.selTiles];
}

/** Keys while ATLAS is shown. Returns true when the key was handled. */
export function atlasKey(e) {
    if (!visible) return false;
    const ctrl = e.ctrlKey || e.metaKey;
    if (ctrl && e.code === 'KeyS') {
        e.preventDefault();
        if (!e.repeat) (e.shiftKey ? saveAs() : save());
    } else if (ctrl && e.code === 'KeyO') {
        e.preventDefault();
        openFile();
    } else if (ctrl && e.code === 'KeyN') {
        e.preventDefault();
        newWorkspace();
    } else if (e.code === 'KeyR' && !ctrl && !e.altKey) {
        e.preventDefault();
        if (canvas.selTiles.size) edit({ op: 'rotate', ids: selectedTiles(), clockwise: !e.shiftKey });
    } else if (e.key === 'Delete') {
        e.preventDefault();
        if (canvas.selTiles.size) edit({ op: 'delete', ids: selectedTiles() });
    } else if (e.key === 'Escape') {
        // In ATLAS, Esc only clears the selection; it never closes the app.
        e.preventDefault();
        canvas.selTiles.clear();
        canvas.selSectors = [];
        canvas.request();
        render();
    } else if (e.code === 'KeyF' && !ctrl) {
        canvas.fit();
    } else if (e.key === ' ') {
        e.preventDefault();
        if (!e.repeat) canvas.setSpace(true);
    } else {
        return false;
    }
    return true;
}

export function atlasKeyUp(e) {
    if (e.key === ' ' && canvas) canvas.setSpace(false);
}

async function poll() {
    if (!visible || !doc || busy || canvas.busy) return;
    try {
        const rev = await invoke('atlas_revision');
        if (rev !== doc.manifest.revision) await reload();
    } catch {
        /* the next poll retries */
    }
}

export async function showAtlas() {
    visible = true;
    el('atlasStage').hidden = false;
    canvas.resize();
    await reload();
    canvas.fitted = false;
    canvas.fit();
    polling ??= setInterval(poll, 500);
}

export function hideAtlas() {
    visible = false;
    el('atlasStage').hidden = true;
    clearInterval(polling);
    polling = null;
}

export function initAtlas() {
    canvas = new AtlasCanvas(el('atlasStage'), el('atlasCanvas'), el('atlasPlus'), {
        edit,
        select: render,
        status: (t) => setStatus(t),
    });
    el('atlasNewBtn').addEventListener('click', newWorkspace);
    el('atlasOpenBtn').addEventListener('click', openFile);
    el('atlasSaveBtn').addEventListener('click', save);
    el('atlasSaveAsBtn').addEventListener('click', saveAs);
    el('atlasSizeSeg').addEventListener('click', (e) => {
        const b = e.target.closest('.seg');
        if (b && !b.disabled) edit({ op: 'set_size', size: Number(b.dataset.size) });
    });
    el('atlasPadding').addEventListener('change', () => {
        const v = Number(el('atlasPadding').value);
        if (Number.isInteger(v) && v >= 0) edit({ op: 'set_padding', padding: v });
        else render();
    });
    el('atlasRemoveSectorBtn').addEventListener('click', () => {
        if (canvas.selSectors.length === 1) edit({ op: 'remove_sector', sector: canvas.selSectors[0] });
    });
    el('atlasScopeSeg').addEventListener('click', (e) => {
        const b = e.target.closest('.seg');
        if (!b) return;
        scope = b.dataset.scope;
        render();
    });
    el('atlasExportName').addEventListener('input', () => {
        nameTouched = true;
    });
    el('atlasExportBtn').addEventListener('click', exportSectors);
}
