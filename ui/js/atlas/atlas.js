// ATLAS mode (OpenSpec change add-texture-atlas): panels, workspace commands, change polling
// and keys. Rust owns the workspace; every command returns the new document (design D3).

import { setStatus } from '../status.js';
import { AtlasCanvas, sectorRect } from './canvas.js';
import * as F from './frame.js';
import { setNameField } from '../name-field.js';

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

// Undo history of this window (design D6 of add-atlas-fine-work): each step is the list of
// edits that reverses it, computed by Rust. Lost with the window; cleared by NEW and OPEN.
const HISTORY = 100;
let undoStack = [];
let redoStack = [];

const fileName = (p) => (p ? p.split(/[\\/]/).pop() : null);

function tilesOn(sector) {
    const m = doc.manifest;
    const r = sectorRect(sector, m.sector_size);
    return m.tiles.some((t) => {
        const b = F.rect(t);
        return b.x < r.x + r.w && r.x < b.x + b.w && b.y < r.y + r.h && r.y < b.y + b.h;
    });
}

function pushStep(label, edits) {
    if (!edits || edits.length === 0) return;
    undoStack.push({ label, edits });
    if (undoStack.length > HISTORY) undoStack.shift();
    redoStack = [];
}

function clearHistory() {
    undoStack = [];
    redoStack = [];
}

/** Undo (from = undoStack) or redo (from = redoStack): applies the step best effort. */
async function step(from, to, verb) {
    if (canvas.busy) return;
    const s = from.pop();
    if (!s) {
        setStatus(verb === 'Undo' ? 'Nothing to undo' : 'Nothing to redo');
        return;
    }
    try {
        const r = await invoke('atlas_apply_steps', { edits: s.edits });
        setDoc(r.doc);
        if (r.inverse.length) to.push({ label: s.label, edits: r.inverse });
        if (r.skipped.length) setStatus(`${verb}: ${s.label}; ${r.skipped.join('; ')}`, 'error');
        else setStatus(`${verb}: ${s.label}`, 'success');
    } catch (e) {
        from.push(s);
        setStatus(errText(e), 'error');
    }
}

const undo = () => step(undoStack, redoStack, 'Undo');
const redo = () => step(redoStack, undoStack, 'Redo');

function udimOf(sector) {
    const m = doc.manifest;
    const umin = Math.min(...m.sectors.map((s) => s.u));
    const vmin = Math.min(...m.sectors.map((s) => s.v));
    return 1001 + (sector.u - umin) + 10 * (sector.v - vmin);
}

function render() {
    if (!doc) return;
    const m = doc.manifest;
    setNameField(el('atlasName'), {
        value: m.name,
        editable: !doc.file,
        file: doc.file,
        commit: async (name) => {
            try {
                setDoc(await invoke('atlas_rename', { name }));
                setStatus(`Workspace named ${name}`, 'success');
                return true;
            } catch (e) {
                setStatus(errText(e), 'error');
                return false;
            }
        },
    });
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
    const one = sel.length === 1 ? sel[0] : null;
    const guide = one && m.guides.find((g) => g.sector.u === one.u && g.sector.v === one.v);
    el('atlasGuideSector').textContent = one ? String(udimOf(one)) : '';
    el('atlasGuideRemoveBtn').disabled = !guide;
    el('atlasGuideShow').checked = m.guide_show;
    if (canvas.opacity === null) setOpacityControls(m.guide_opacity);
    for (const b of el('atlasFillSeg').querySelectorAll('.seg')) b.classList.toggle('is-active', b.dataset.fill === m.fill);
    el('atlasEdge').disabled = m.fill !== 'extend';
    el('atlasEdge').max = String(m.sector_size);
    if (document.activeElement !== el('atlasEdge')) el('atlasEdge').value = String(m.edge);

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
        pushStep(r.label, r.inverse);
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
    const d = await run(() => invoke('atlas_open', { path }), (d) => `Opened ${fileName(d.file)}${d.unsaved ? ' with unsaved changes restored' : ''}`);
    if (d) clearHistory();
}

async function newWorkspace() {
    if (await run(() => invoke('atlas_new'), 'New workspace')) clearHistory();
}

// ---- Guides panel (add-atlas-fine-work) ----

function setOpacityControls(v) {
    el('atlasGuideOpacity').value = String(v);
    const range = el('atlasGuideOpacityRange');
    range.value = String(v);
    range.style.setProperty('--fill-start', '0%');
    range.style.setProperty('--fill-end', `${v}%`);
}

function commitOpacity(v) {
    const o = Math.min(100, Math.max(0, Math.round(Number(v))));
    if (!Number.isFinite(o)) return render();
    canvas.opacity = null;
    setOpacityControls(o);
    if (o !== doc.manifest.guide_opacity) edit({ op: 'set_guide_view', show: null, opacity: o });
    canvas.request();
}

async function loadGuides() {
    const paths = await window.__TAURI__.dialog.open({
        title: 'Load UV-layout guides',
        multiple: true,
        directory: false,
        filters: [{ name: 'PNG', extensions: ['png'] }],
    });
    const list = Array.isArray(paths) ? paths : typeof paths === 'string' ? [paths] : [];
    if (!list.length) return;
    const target = canvas.selSectors.length === 1 ? canvas.selSectors[0] : null;
    try {
        const r = await invoke('atlas_load_guides', { paths: list, target });
        setDoc(r.doc);
        pushStep(r.label, r.inverse);
        let text = `Loaded ${r.loaded} guide${r.loaded === 1 ? '' : 's'}`;
        if (r.created) text += `, created ${r.created} sector${r.created === 1 ? '' : 's'}`;
        if (r.not_square) {
            text += `; ${r.not_square === 1 ? 'one guide is' : `${r.not_square} guides are`} not square and stretched to the sector`;
            setStatus(text, 'error');
        } else {
            setStatus(text, 'success');
        }
    } catch (e) {
        setStatus(errText(e), 'error');
    }
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
    if (e.key === 'Alt') {
        // Alt moves the pivot; keep WebView2 from moving focus to the window chrome.
        e.preventDefault();
    } else if (ctrl && e.code === 'KeyZ') {
        e.preventDefault();
        if (!e.repeat) (e.shiftKey ? redo() : undo());
    } else if (ctrl && e.code === 'KeyY') {
        e.preventDefault();
        if (!e.repeat) redo();
    } else if (e.code === 'KeyG' && !ctrl && !e.altKey) {
        e.preventDefault();
        canvas.setPreview(true);
    } else if (ctrl && e.code === 'KeyS') {
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
    if (e.code === 'KeyG' && canvas) canvas.setPreview(false);
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

    el('atlasGuideLoadBtn').addEventListener('click', loadGuides);
    el('atlasGuideRemoveBtn').addEventListener('click', () => {
        if (canvas.selSectors.length === 1) edit({ op: 'remove_guide', sector: canvas.selSectors[0] });
    });
    el('atlasGuideShow').addEventListener('change', () => edit({ op: 'set_guide_view', show: el('atlasGuideShow').checked, opacity: null }));
    el('atlasGuideOpacityRange').addEventListener('input', () => {
        // Live while dragging; written once on release.
        canvas.opacity = Number(el('atlasGuideOpacityRange').value);
        setOpacityControls(canvas.opacity);
        canvas.request();
    });
    el('atlasGuideOpacityRange').addEventListener('change', () => commitOpacity(el('atlasGuideOpacityRange').value));
    el('atlasGuideOpacity').addEventListener('change', () => commitOpacity(el('atlasGuideOpacity').value));
    el('atlasGuideOpacityReset').addEventListener('click', () => commitOpacity(50));
    el('atlasFillSeg').addEventListener('click', (e) => {
        const b = e.target.closest('.seg');
        if (b && b.dataset.fill !== doc.manifest.fill) edit({ op: 'set_fill', fill: b.dataset.fill, edge: doc.manifest.edge });
    });
    el('atlasEdge').addEventListener('change', () => {
        const v = Number(el('atlasEdge').value);
        if (Number.isInteger(v) && v >= 1 && v <= doc.manifest.sector_size) edit({ op: 'set_fill', fill: doc.manifest.fill, edge: v });
        else {
            setStatus(`Edge must be from 1 to ${doc.manifest.sector_size} px`, 'error');
            render();
        }
    });
    // The G preview ends when the window loses focus (the key-up never arrives).
    window.addEventListener('blur', () => canvas.setPreview(false));
}
