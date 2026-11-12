// Output panel: two folder slots (Local, Google Drive), each with an on/off checkbox, a
// read-only path field and a browse button. Changes are saved immediately.

import { saveSettings, settings } from './settings.js';
import { setStatus } from './status.js';

const SLOTS = [
    { on: 'localOn', dir: 'localDir', browse: 'localBrowse', onKey: 'local_on', dirKey: 'local_dir', title: 'Choose the local output folder' },
    { on: 'driveOn', dir: 'driveDir', browse: 'driveBrowse', onKey: 'drive_on', dirKey: 'drive_dir', title: 'Choose the Google Drive output folder' },
];

const el = (id) => document.getElementById(id);

export function renderOutput() {
    if (!settings) return;
    for (const s of SLOTS) {
        el(s.on).checked = settings[s.onKey];
        el(s.dir).value = settings[s.dirKey];
        el(s.dir).title = settings[s.dirKey];
    }
}

async function save(patch) {
    const err = await saveSettings(patch);
    if (err) setStatus(err, 'error');
    renderOutput();
}

export function initOutput() {
    for (const s of SLOTS) {
        el(s.on).addEventListener('change', () => save({ [s.onKey]: el(s.on).checked }));
        el(s.browse).addEventListener('click', async () => {
            const dir = await window.__TAURI__.dialog.open({
                directory: true,
                multiple: false,
                title: s.title,
                defaultPath: settings[s.dirKey] || undefined,
            });
            if (typeof dir === 'string') await save({ [s.dirKey]: dir });
        });
    }
    renderOutput();
}
