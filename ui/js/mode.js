// RIP | ATLAS mode switch (design D6 of add-texture-atlas). Each mode owns its stage and
// panels; switching keeps the other mode's state. A ShareX launch always starts in RIP.

import { hideAtlas, showAtlas } from './atlas/atlas.js';

let mode = 'rip';
let onRip = () => {};

export const currentMode = () => mode;

export async function setMode(next) {
    if (next === mode) return;
    mode = next;
    for (const b of document.getElementById('modeSeg').querySelectorAll('.seg')) b.classList.toggle('is-active', b.dataset.mode === mode);
    document.getElementById('ripPanels').hidden = mode !== 'rip';
    document.getElementById('atlasPanels').hidden = mode !== 'atlas';
    document.getElementById('stage').hidden = mode !== 'rip';
    if (mode === 'atlas') await showAtlas();
    else {
        hideAtlas();
        onRip();
    }
}

/** `ripShown` runs when RIP becomes visible again (resize and redraw its canvases). */
export function initMode(ripShown, enabled) {
    onRip = ripShown;
    const seg = document.getElementById('modeSeg');
    if (!enabled) {
        // Outside the app (?demo) there is no atlas backend.
        for (const b of seg.querySelectorAll('.seg')) b.disabled = b.dataset.mode === 'atlas';
        return;
    }
    seg.addEventListener('click', (e) => {
        const b = e.target.closest('.seg');
        if (b) setMode(b.dataset.mode);
    });
}
