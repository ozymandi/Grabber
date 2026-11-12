// Aspect panel: EDGES / presets / CUSTOM and Power of two for the active quad. The last
// choice also becomes the default for new quads (persisted with the settings in group 8).

import { activeQuad, emit, state } from './state.js';

const seg = document.getElementById('aspectSeg');
const custom = document.getElementById('customRatio');
const ratioW = document.getElementById('ratioW');
const ratioH = document.getElementById('ratioH');
const pot = document.getElementById('pot');

let onDefaultsChanged = () => {};

/** Current settings shown by the panel: the active quad's, or the defaults when none. */
function current() {
    return activeQuad() ?? state.defaults;
}

export function renderAspect() {
    const c = current();
    for (const b of seg.querySelectorAll('.seg')) b.classList.toggle('is-active', b.dataset.aspect === c.aspectKey);
    custom.hidden = c.aspectKey !== 'custom';
    if (document.activeElement !== ratioW) ratioW.value = String(c.custom.w);
    if (document.activeElement !== ratioH) ratioH.value = String(c.custom.h);
    pot.checked = c.pot;
}

function update(patch) {
    const q = activeQuad();
    if (q) Object.assign(q, patch);
    Object.assign(state.defaults, patch);
    if (patch.custom) state.defaults.custom = { ...patch.custom };
    onDefaultsChanged(state.defaults);
    emit('geometry');
}

function readRatio() {
    const w = Number(ratioW.value);
    const h = Number(ratioH.value);
    return w > 0 && h > 0 && Number.isFinite(w) && Number.isFinite(h) ? { w, h } : null;
}

/** `changed` is called with the new defaults whenever the user changes them. */
export function initAspect(changed) {
    onDefaultsChanged = changed;
    seg.addEventListener('click', (e) => {
        const b = e.target.closest('.seg');
        if (b) update({ aspectKey: b.dataset.aspect });
    });
    for (const input of [ratioW, ratioH]) {
        input.addEventListener('input', () => {
            const r = readRatio();
            if (r) update({ custom: r });
        });
        input.addEventListener('blur', () => renderAspect());
    }
    pot.addEventListener('change', () => update({ pot: pot.checked }));
    renderAspect();
}
