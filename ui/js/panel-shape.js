// Shape picker in the Quads panel header (design D9 of add-quad-shapes): an icon showing
// the active quad's shape (or the shape for new quads) that opens a list of the four shapes,
// with N for the N-gon. Shift+S cycles the shapes (main.js).

import { SHAPE_ICONS } from './icons.js';
import { KINDS, NGON_MAX, NGON_MIN } from './shape.js';
import { activeQuad, setShape, state } from './state.js';

const pick = document.getElementById('shapePick');
const btn = document.getElementById('shapeBtn');
const menu = document.getElementById('shapeMenu');

const NAMES = { rect: 'Rectangle', circle: 'Circle', ngon: 'N-gon', path: 'Path' };
let nInput = null;

/** Shape kind and N shown by the picker. */
function current() {
    const q = activeQuad();
    const d = state.defaults;
    if (!q) return { kind: d.shapeKind, n: d.ngon };
    return { kind: q.shape.kind, n: q.shape.kind === 'ngon' ? q.shape.n : d.ngon };
}

export function renderShape() {
    const c = current();
    btn.innerHTML = SHAPE_ICONS[c.kind];
    const label = c.kind === 'ngon' ? `${NAMES.ngon} ${c.n}` : NAMES[c.kind];
    btn.title = `Shape: ${label} (Shift+S for the next)`;
    btn.setAttribute('aria-label', `Shape: ${label}`);
    for (const o of menu.querySelectorAll('.shape-option')) o.classList.toggle('is-selected', o.dataset.kind === c.kind);
    if (nInput && document.activeElement !== nInput) nInput.value = String(c.n);
}

export const isShapeMenuOpen = () => pick.classList.contains('open');

export function closeShapeMenu() {
    if (!isShapeMenuOpen()) return false;
    pick.classList.remove('open');
    btn.setAttribute('aria-expanded', 'false');
    return true;
}

function openMenu() {
    pick.classList.add('open');
    btn.setAttribute('aria-expanded', 'true');
    renderShape();
}

const readN = () => {
    const n = Math.round(Number(nInput.value));
    return Number.isFinite(n) ? Math.min(Math.max(n, NGON_MIN), NGON_MAX) : null;
};

/** Shift+S: the next shape for the active quad (spec "Cycle with Shift+S"). */
export function cycleShape() {
    const c = current();
    setShape(KINDS[(KINDS.indexOf(c.kind) + 1) % KINDS.length], c.n);
}

export function initShape() {
    menu.replaceChildren(
        ...KINDS.map((kind) => {
            const row = document.createElement('div');
            row.className = 'custom-option shape-option';
            row.dataset.kind = kind;
            row.setAttribute('role', 'menuitemradio');
            row.innerHTML = `${SHAPE_ICONS[kind]}<span class="shape-name">${NAMES[kind]}</span>`;
            if (kind === 'ngon') {
                nInput = document.createElement('input');
                nInput.type = 'number';
                nInput.min = String(NGON_MIN);
                nInput.max = String(NGON_MAX);
                nInput.step = '1';
                nInput.title = `Number of sides, ${NGON_MIN} to ${NGON_MAX}`;
                nInput.addEventListener('click', (e) => e.stopPropagation());
                nInput.addEventListener('change', () => {
                    const n = readN();
                    if (n !== null) setShape('ngon', n);
                    renderShape();
                });
                nInput.addEventListener('keydown', (e) => {
                    if (e.key === 'Escape') {
                        e.preventDefault();
                        e.stopPropagation();
                        nInput.blur();
                        closeShapeMenu();
                    } else if (e.key === 'Enter') {
                        e.preventDefault();
                        nInput.blur();
                        closeShapeMenu();
                    }
                });
                row.append(nInput);
            }
            row.addEventListener('click', () => {
                setShape(kind, kind === 'ngon' ? (readN() ?? state.defaults.ngon) : state.defaults.ngon);
                closeShapeMenu();
            });
            return row;
        }),
    );
    btn.addEventListener('click', (e) => {
        e.stopPropagation();
        if (!closeShapeMenu()) openMenu();
    });
    document.addEventListener('pointerdown', (e) => {
        if (isShapeMenuOpen() && !pick.contains(e.target)) closeShapeMenu();
    });
    renderShape();
}
