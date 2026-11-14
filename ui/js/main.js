import { copyActive, quit, saveAll, sendActive } from './actions.js';
import { Editor } from './editor.js';
import { initHelp, isHelpOpen } from './help.js';
import { initAspect, renderAspect } from './panel-aspect.js';
import { initOutput } from './panel-output.js';
import { renderQuads } from './panel-quads.js';
import { closeShapeMenu, cycleShape, initShape, renderShape } from './panel-shape.js';
import { Preview } from './preview.js';
import { loadSettings, saveDefaults } from './settings.js';
import { fetchSource, getLaunch, inApp, loadPath, log, pasteClipboard } from './source.js';
import { redo, undo } from './history.js';
import { onChange, removeQuad, setActive, setSource, state } from './state.js';
import { initUpdate, tauriUpdateSource } from './update.js';
import { atlasKey, atlasKeyUp, initAtlas } from './atlas/atlas.js';
import { currentMode, initMode } from './mode.js';
import { refreshAtlasTarget } from './panel-output.js';
import { setStatus } from './status.js';
import { View } from './view.js';

const stage = document.getElementById('stage');
const view = new View(document.getElementById('view'));
const editor = new Editor(stage, view, document.getElementById('overlay'));
const preview = new Preview();

onChange((what) => {
    if (what !== 'draft') {
        renderQuads();
        renderAspect();
        renderShape();
        preview.request();
    }
    if (what === 'shape' && inApp()) {
        saveDefaults(state.defaults).then((err) => err && setStatus(err, 'error'));
    }
    if (what === 'draft') {
        const n = state.draft.length;
        if (n > 0) setStatus(`Corner ${n + 1} of 4`);
    }
    if (what === 'quads' && state.draft.length === 0 && state.source) sourceStatus();
    editor.request();
});

function sourceStatus() {
    const s = state.source;
    setStatus(`${s.stem}  ${s.w} × ${s.h}  ·  ${state.quads.length} quad${state.quads.length === 1 ? '' : 's'}`);
}

/** Pulls the pixels of a source Rust has just loaded and resets the canvas to it. */
async function showSource(info) {
    const src = await fetchSource(info);
    log(`source ${src.w}x${src.h} stem=${src.stem} fetched in ${src.ms.toFixed(1)} ms`);
    setSource(src);
    view.setImage(src);
    preview.setImage(src);
    editor.fit();
    sourceStatus();
}

async function replaceSource(load) {
    try {
        await showSource(await load());
    } catch (e) {
        setStatus(String(e).replace(/^Error: /, ''), 'error');
    }
}

const openFile = async () => {
    const path = await window.__TAURI__.dialog.open({
        multiple: false,
        directory: false,
        filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'bmp', 'webp'] }],
    });
    if (typeof path === 'string') await replaceSource(() => loadPath(path));
};

const paste = () => replaceSource(pasteClipboard);

function isTyping(e) {
    const t = e.target;
    return t instanceof HTMLInputElement && (t.type === 'text' || t.type === 'number');
}

function onKey(e) {
    if (isHelpOpen()) return;
    if (currentMode() === 'atlas') {
        // ATLAS has its own keys; RIP keys (Enter, Ctrl+V, Tab...) do nothing there.
        if (!isTyping(e)) atlasKey(e);
        return;
    }
    if (e.key === ' ' && !isTyping(e)) {
        e.preventDefault();
        if (!e.repeat) editor.setSpace(true);
        return;
    }
    if (isTyping(e)) return;
    const ctrl = e.ctrlKey || e.metaKey;
    // Letter shortcuts use the physical key (e.code), so they work in any keyboard
    // layout: in Ukrainian, V/S/F type "м"/"і"/"а" and e.key never matches.
    if (ctrl && (e.code === 'KeyZ' || e.code === 'KeyY')) {
        // Undo and redo of quad edits (spec rip-undo), as in ATLAS.
        e.preventDefault();
        if (e.repeat) return;
        if (e.code === 'KeyY' || e.shiftKey) {
            if (!redo()) setStatus('Nothing to redo');
        } else if (!undo()) setStatus('Nothing to undo');
    } else if (ctrl && e.code === 'KeyV') {
        e.preventDefault();
        paste();
    } else if (ctrl && e.code === 'KeyS') {
        e.preventDefault();
        if (!e.repeat) saveAll();
    } else if (e.key === 'Enter' && !ctrl && !e.altKey) {
        e.preventDefault();
        if (!e.repeat) sendActive();
    } else if (!ctrl && e.shiftKey && !e.altKey && e.code === 'KeyS') {
        e.preventDefault();
        if (!e.repeat) cycleShape();
    } else if (e.key === 'Escape') {
        e.preventDefault();
        // Esc closes the shape menu or cancels an unfinished quad; otherwise it closes the
        // app without writing anything.
        if (closeShapeMenu() || editor.cancelPen()) return;
        if (editor.cancelDraft()) sourceStatus();
        else if (inApp()) quit();
    } else if (e.key === 'Delete') {
        e.preventDefault();
        removeQuad(state.active);
    } else if (e.key === 'Tab') {
        e.preventDefault();
        if (state.quads.length > 0) {
            const step = e.shiftKey ? -1 : 1;
            setActive((state.active + step + state.quads.length) % state.quads.length);
        }
    } else if (e.code === 'KeyP' && !ctrl && !e.altKey) {
        // P redraws the active Path quad's outline with the Pen (spec "Draw a path with the Pen").
        e.preventDefault();
        if (!e.repeat) editor.startPen();
    } else if (e.code === 'KeyF') {
        if (!ctrl) editor.fit();
    }
}

function wireUi() {
    initHelp();
    initShape();
    window.addEventListener('keydown', onKey);
    window.addEventListener('keyup', (e) => {
        if (e.key === ' ') editor.setSpace(false);
        atlasKeyUp(e);
    });
    window.addEventListener('blur', () => editor.setSpace(false));
    document.getElementById('openBtn').addEventListener('click', () => openFile());
    document.getElementById('pasteBtn').addEventListener('click', () => paste());
    document.getElementById('sendBtn').addEventListener('click', () => sendActive());
    document.getElementById('saveAllBtn').addEventListener('click', () => saveAll());
    document.getElementById('copyBtn').addEventListener('click', () => copyActive());
    renderQuads();
}

/** Shows the window once the page has painted, so it never flashes white (design D8). */
function revealWindow() {
    requestAnimationFrame(() =>
        requestAnimationFrame(async () => {
            await log('ready');
            await window.__TAURI__.window.getCurrentWindow().show();
        }),
    );
}

/** RIP is visible again after ATLAS: refit its canvases and refresh the Atlas slot. */
function ripShown() {
    editor.resize();
    editor.request();
    preview.request();
    refreshAtlasTarget();
}

async function start() {
    wireUi();
    initMode(ripShown, inApp());
    if (inApp()) initAtlas();
    if (!inApp()) {
        initAspect(() => {});
        setStatus('Open, paste or drop an image');
        if (new URLSearchParams(location.search).has('demo')) {
            const { demoSource } = await import('./dev.js');
            const src = demoSource();
            setSource(src);
            view.setImage(src);
            preview.setImage(src);
            editor.fit();
            sourceStatus();
            window.__ripper = { state, view, editor, preview };
            const demoVersion = new URLSearchParams(location.search).get('update');
            if (demoVersion) {
                const { demoUpdateSource } = await import('./dev.js');
                await initUpdate(demoUpdateSource(demoVersion));
            }
        }
        return;
    }
    const settingsWarning = await loadSettings().catch((e) => String(e));
    initAspect(async (d) => {
        const err = await saveDefaults(d);
        if (err) setStatus(err, 'error');
    });
    initOutput();
    revealWindow();
    window.__TAURI__.webview.getCurrentWebview().onDragDropEvent((ev) => {
        if (ev.payload.type === 'drop' && ev.payload.paths?.length) {
            replaceSource(() => loadPath(ev.payload.paths[0]));
        }
    });
    const launch = await getLaunch();
    state.mode = launch.mode;
    log(`launch mode=${launch.mode} output=${launch.output ?? '-'} error=${launch.error ?? '-'}`);
    if (launch.error) setStatus(launch.error, 'error');
    else if (launch.source) await showSource(launch.source);
    else setStatus('Open, paste or drop an image');
    if (settingsWarning) setStatus(settingsWarning, 'error');
    // Rust only ever reports an update in standalone mode (spec app-distribution).
    if (state.mode === 'standalone') await initUpdate(tauriUpdateSource());
}

start().catch((e) => {
    setStatus(String(e), 'error');
    log(`start failed: ${e}`);
});
