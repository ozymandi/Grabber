// Self-update UI (spec app-distribution; design D4 of add-installer-updater). Rust runs the
// check and the install and refuses both in ShareX mode; this module only shows the button,
// the status lines and the unsaved-quads confirmation.

import { UPDATE_ICON } from './icons.js';
import { onChange, unsavedCount } from './state.js';
import { setStatus, statusText } from './status.js';

const btn = document.getElementById('updateBtn');

let api = null;
let version = null;
/** Text of the unsaved-quads warning while it waits for a confirming click. */
let armedText = null;
let installing = false;
let lastPct = -1;

function available(v) {
    if (!v || version) return;
    version = v;
    btn.title = `Update to ${v}`;
    btn.setAttribute('aria-label', `Update to ${v}`);
    btn.hidden = false;
    setStatus(`Update ${v} available`);
}

function progress({ done, total }) {
    if (!installing) return;
    if (total) {
        const pct = Math.min(100, Math.floor((done / total) * 100));
        if (pct === lastPct) return;
        lastPct = pct;
        setStatus(`Downloading update ${version}: ${pct}%`);
    } else {
        setStatus(`Downloading update ${version}: ${(done / 1048576).toFixed(1)} MB`);
    }
}

async function onClick() {
    if (installing || !version) return;
    const n = unsavedCount();
    // The warning counts as confirmed only while it is still the status text; any other
    // message or a quad change in between disarms it.
    if (n > 0 && !(armedText && statusText() === armedText)) {
        armedText = `${n} unsaved quad${n === 1 ? '' : 's'} will be lost. Click update again to install`;
        setStatus(armedText, 'error');
        return;
    }
    armedText = null;
    installing = true;
    lastPct = -1;
    setStatus(`Downloading update ${version}`);
    try {
        // On success the installer takes over and the app exits; this returns only on failure.
        await api.install();
    } catch (e) {
        setStatus(String(e).replace(/^Error: /, ''), 'error');
    } finally {
        installing = false;
    }
}

/**
 * `source` supplies `listen(event, handler)`, `getUpdate()` and `install()`: Tauri's in the
 * app, a stub from dev.js in the `?demo` page.
 */
export async function initUpdate(source) {
    api = source;
    btn.innerHTML = UPDATE_ICON;
    btn.addEventListener('click', onClick);
    onChange((what) => {
        if (what !== 'draft' && what !== 'active') armedText = null;
    });
    await api.listen('update-available', (p) => available(p.version));
    await api.listen('update-progress', progress);
    await api.listen('update-installing', () => setStatus(`Verifying update ${version}`));
    available(await api.getUpdate());
}

/** The app's own source: Rust events and commands. */
export function tauriUpdateSource() {
    const { core, event } = window.__TAURI__;
    return {
        listen: (name, fn) => event.listen(name, (e) => fn(e.payload ?? {})),
        getUpdate: () => core.invoke('get_update'),
        install: () => core.invoke('install_update'),
    };
}
