// History panel of ATLAS mode (OpenSpec change add-mcp-editing, spec atlas-history, design D8):
// the picture states of the selected tile, or the LLM edits of the selected sector, newest on
// top. A click goes to that state (not an undo step); Ctrl+C on the last clicked row, or a
// right-click on any row, copies it; Ctrl+V in ATLAS pastes it as new tiles.

import { sectorRect } from './canvas.js';

const el = (id) => document.getElementById(id);
const THUMB = 48; // CSS px; the request asks for 2x for sharp rows

let rows = []; // what each listed row stands for, in list order
let chosen = null; // the row last clicked, for Ctrl+C
let clip = null; // copied paste payload: { label, tiles: [PasteTile] }

function when(at) {
    if (!at) return '';
    const [date, time] = at.split('T');
    const today = new Date();
    const pad = (n) => String(n).padStart(2, '0');
    const iso = `${today.getFullYear()}-${pad(today.getMonth() + 1)}-${pad(today.getDate())}`;
    const hm = (time || '').slice(0, 5);
    if (date === iso) return hm;
    const [, m, d] = date.split('-');
    return `${d}.${m} ${hm}`;
}

const same = (a, b) => a.u === b.u && a.v === b.v;

/** Rows for one tile: its states, newest on top. */
function tileRows(t) {
    const states = t.history?.length ? t.history : [{ item: t.item, w0: t.w0, h0: t.h0, name: 'Original', by: 'RIP', at: '' }];
    return states
        .map((s, i) => ({ kind: 'tile', tile: t.id, index: i, item: s.item, pixel: s.filter === 'nearest', name: s.name, meta: [s.by, when(s.at)].filter(Boolean).join(' · '), current: i === (t.current ?? 0) }))
        .reverse();
}

/** Rows for one sector: its LLM edits newest on top, and "Before ..." under the oldest. */
function sectorRows(m, sector) {
    const entries = m.sector_log.map((e, index) => ({ ...e, index })).filter((e) => same(e.sector, sector));
    if (!entries.length) return [];
    const byId = new Map(m.tiles.map((t) => [t.id, t]));
    const at = (e, before) => e.tiles.filter((c) => byId.has(c.id)).every((c) => (byId.get(c.id).current ?? 0) === (before ? c.before : c.after));
    const out = entries.map((e) => {
        const first = e.tiles.map((c) => byId.get(c.id)).find(Boolean);
        const st = first?.history?.[e.tiles.find((c) => c.id === first.id).after];
        return { kind: 'entry', entry: e.index, before: false, item: st?.item, pixel: st?.filter === 'nearest', name: e.name, meta: [`${e.tiles.length} tile${e.tiles.length === 1 ? '' : 's'}`, e.by, when(e.at)].filter(Boolean).join(' · '), current: false };
    });
    const oldest = entries[0];
    const firstOld = oldest.tiles.map((c) => byId.get(c.id)).find(Boolean);
    const beforeItem = firstOld?.history?.[oldest.tiles.find((c) => c.id === firstOld.id).before]?.item;
    out.unshift({ kind: 'entry', entry: oldest.index, before: true, item: beforeItem, name: `Before ${oldest.name}`, meta: '', current: false });
    // Current: the newest row whose tiles all show what it wrote.
    const flags = [at(oldest, true), ...entries.map((e) => at(e, false))];
    const k = flags.lastIndexOf(true);
    if (k >= 0) out[k].current = true;
    return out.reverse();
}

/** Re-renders the panel for the current selection. */
export function renderHistory(doc, canvas) {
    const list = el('atlasHistoryList');
    const what = el('atlasHistoryWhat');
    const hint = el('atlasHistoryHint');
    const m = doc?.manifest;
    rows = [];
    let label = '';
    if (m && canvas.selTiles.size === 1) {
        const id = [...canvas.selTiles][0];
        const t = m.tiles.find((x) => x.id === id);
        if (t) {
            rows = tileRows(t);
            label = `Tile ${id}`;
        }
    } else if (m && canvas.selTiles.size === 0 && canvas.selSectors.length === 1) {
        const s = canvas.selSectors[0];
        rows = sectorRows(m, s);
        const umin = Math.min(...m.sectors.map((c) => c.u));
        const vmin = Math.min(...m.sectors.map((c) => c.v));
        label = `Sector ${1001 + (s.u - umin) + 10 * (s.v - vmin)}`;
        if (!rows.length) hint.textContent = 'No LLM edits on this sector yet';
    }
    what.textContent = label;
    if (!label) hint.textContent = 'Select one tile or one sector to see its history';
    hint.hidden = rows.length > 0;
    if (!rows.some((r) => chosen && r.kind === chosen.kind && r.tile === chosen.tile && r.entry === chosen.entry && r.index === chosen.index && r.before === chosen.before)) chosen = null;
    list.replaceChildren(
        ...rows.map((r, i) => {
            const li = document.createElement('li');
            li.className = 'history-row' + (r.current ? ' is-active' : '') + (chosen === r ? ' is-chosen' : '');
            li.tabIndex = 0;
            li.dataset.row = String(i);
            li.title = r.kind === 'tile' ? 'Click to show this picture; right-click or Ctrl+C to copy it' : 'Click to put the sector back to this edit; right-click or Ctrl+C to copy it';
            const img = document.createElement('img');
            img.className = 'history-thumb' + (r.pixel ? ' is-pixel' : '');
            img.alt = '';
            img.width = THUMB;
            img.height = THUMB;
            if (r.item) img.src = `http://ripper.localhost/atlas/${doc.id}/${r.item}?max=${THUMB * 2}`;
            const text = document.createElement('div');
            text.className = 'history-text';
            const name = document.createElement('span');
            name.className = 'history-name';
            name.textContent = r.name;
            const meta = document.createElement('span');
            meta.className = 'history-meta';
            meta.textContent = r.meta;
            text.append(name, meta);
            li.append(img, text);
            return li;
        }),
    );
}

/** The row behind a click target, if any. */
function rowOf(target) {
    const li = target.closest?.('.history-row');
    return li ? rows[Number(li.dataset.row)] : null;
}

/** Paste payload for a row: the picture(s) and the unrotated frame size of their source. */
function payload(doc, r) {
    const m = doc.manifest;
    const byId = new Map(m.tiles.map((t) => [t.id, t]));
    if (r.kind === 'tile') {
        const t = byId.get(r.tile);
        const s = t.history?.[r.index] ?? { item: t.item, w0: t.w0, h0: t.h0, name: 'Original', by: 'RIP', at: '', filter: 'smooth' };
        return { label: s.name, tiles: [{ state: s, w: t.w, h: t.h, offset: null, source: t.source }] };
    }
    const e = m.sector_log[r.entry];
    const cell = sectorRect(e.sector, m.sector_size);
    const tiles = e.tiles
        .filter((c) => byId.has(c.id))
        .map((c) => {
            const t = byId.get(c.id);
            const s = t.history[r.before ? c.before : c.after];
            const x = t.c2[0] / 2 - t.w / 2;
            const y = t.c2[1] / 2 - t.h / 2;
            return { state: s, w: t.w, h: t.h, offset: [Math.round(x - cell.x), Math.round(y - cell.y)], source: t.source };
        });
    return { label: r.name, tiles };
}

/**
 * Wires the panel. `hooks.doc()` gives the current document, `hooks.go(row)` goes to a row's
 * state, `hooks.status(text)` reports a copy.
 */
export function initHistory(hooks) {
    const list = el('atlasHistoryList');
    list.addEventListener('click', (e) => {
        const r = rowOf(e.target);
        if (!r) return;
        chosen = r;
        hooks.go(r);
    });
    list.addEventListener('contextmenu', (e) => {
        const r = rowOf(e.target);
        if (!r) return;
        e.preventDefault();
        copyRow(hooks, r);
    });
}

function copyRow(hooks, r) {
    const p = payload(hooks.doc(), r);
    if (!p.tiles.length) return hooks.status('Nothing to copy: the tiles of this edit are gone');
    clip = p;
    hooks.status(`Copied ${p.label}; Ctrl+V pastes it into the active sector`);
}

/** Ctrl+C in ATLAS: copies the last clicked History row. Returns false when there is none. */
export function copyChosen(hooks) {
    if (!chosen) return false;
    copyRow(hooks, chosen);
    return true;
}

/** The copied payload for Ctrl+V, or null. */
export const copied = () => clip;
