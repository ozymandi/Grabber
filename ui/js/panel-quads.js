// Quads panel: one row per quad with its output size, an invalid marker and a remove icon.

import { REMOVE_ICON } from './icons.js';
import { quadInfo, removeQuad, setActive, state } from './state.js';

const list = document.getElementById('quadList');
const empty = document.getElementById('quadEmpty');
const count = document.getElementById('quadCount');

const REASONS = {
    not_convex: 'Edges cross or the shape is concave',
    too_small: 'Quad is too small',
    collinear: 'Three corners are nearly in a line',
    not_finite: 'Invalid coordinates',
    empty_shape: 'The shape has no area',
};

export const invalidReason = (code) => REASONS[code] ?? code;

export function renderQuads() {
    count.textContent = String(state.quads.length);
    empty.hidden = state.quads.length > 0;
    list.replaceChildren(
        ...state.quads.map((q, i) => {
            const info = quadInfo(q);
            const li = document.createElement('li');
            li.className = 'quad-row' + (i === state.active ? ' is-active' : '');
            li.dataset.index = String(i);

            const name = document.createElement('span');
            name.className = 'quad-name';
            name.textContent = `Q${i + 1}`;

            const size = document.createElement('span');
            size.className = 'quad-size';
            size.textContent = `${info.w} × ${info.h}`;

            const flag = document.createElement('span');
            flag.className = info.invalid ? 'quad-flag quad-flag--invalid' : 'quad-flag';
            if (info.invalid) li.title = invalidReason(info.invalid);

            const remove = document.createElement('button');
            remove.type = 'button';
            remove.className = 'icon-btn';
            remove.title = `Remove Q${i + 1} (Delete)`;
            remove.setAttribute('aria-label', `Remove Q${i + 1}`);
            remove.innerHTML = REMOVE_ICON;
            remove.addEventListener('click', (e) => {
                e.stopPropagation();
                removeQuad(i);
            });

            li.append(name, size, flag, remove);
            li.addEventListener('click', () => setActive(i));
            return li;
        }),
    );
}
