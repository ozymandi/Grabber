// Settings kept by Rust in %APPDATA%\ripper\config.toml (design D10): folders, toggles,
// and the aspect, power-of-two and shape defaults for new quads.

import { invoke } from './source.js';
import { KINDS, NGON_MAX, NGON_MIN } from './shape.js';
import { state } from './state.js';

/** Current settings object, mirrored from Rust. */
export let settings = null;

export async function loadSettings() {
    const info = await invoke('get_settings');
    settings = info.settings;
    const [w, h] = settings.aspect_custom;
    state.defaults = {
        aspectKey: settings.aspect_mode,
        custom: { w, h },
        pot: settings.pot,
        shapeKind: KINDS.includes(settings.shape) ? settings.shape : 'rect',
        ngon: Math.min(Math.max(settings.ngon || 8, NGON_MIN), NGON_MAX),
    };
    return info.warning;
}

/** Applies a partial change and saves. Errors are returned for the status row. */
export async function saveSettings(patch) {
    Object.assign(settings, patch);
    try {
        await invoke('set_settings', { settings });
        return null;
    } catch (e) {
        return String(e);
    }
}

/** Saves the defaults for new quads: aspect, power of two, shape and N. */
export function saveDefaults(d) {
    if (!settings) return Promise.resolve(null);
    return saveSettings({ aspect_mode: d.aspectKey, aspect_custom: [d.custom.w, d.custom.h], pot: d.pot, shape: d.shapeKind, ngon: d.ngon });
}
