// Canvas interaction (spec quad-editing): zoom/pan, creating quads by four clicks, dragging
// corners, activating quads. Draws quads and handles on a 2D overlay above the WebGL view.

import { activeQuad, addQuad, emit, moveCorner, quadInfo, setActive, state } from './state.js';

// O'Tool tokens (otool.css :root).
const C = {
    blue: '#18a0fb',
    muted: '#b3b3b3',
    danger: '#f24822',
    thumb: '#dbeeff',
    halo: 'rgba(0, 0, 0, 0.55)',
    thumbEdge: 'rgba(0, 0, 0, 0.3)',
};

const HANDLE_R = 5; // active-quad handle radius, CSS px (10px, like the slider thumb)
const HANDLE_R_IDLE = 4;
const HIT_R = 8;

function pointInPolygon(p, poly) {
    let inside = false;
    for (let i = 0, j = poly.length - 1; i < poly.length; j = i++) {
        const [xi, yi] = poly[i];
        const [xj, yj] = poly[j];
        if (yi > p[1] !== yj > p[1] && p[0] < ((xj - xi) * (p[1] - yi)) / (yj - yi) + xi) inside = !inside;
    }
    return inside;
}

export class Editor {
    constructor(stage, view, overlay) {
        this.stage = stage;
        this.view = view;
        this.overlay = overlay;
        this.ctx = overlay.getContext('2d');
        this.cursor = null; // last pointer position, CSS px
        this.hover = null; // { q, c } handle under the pointer
        this.drag = null; // { q, c } corner being dragged
        this.pan = null; // { x, y } last pan pointer position
        this.space = false;
        this.frame = 0;

        overlay.addEventListener('pointerdown', (e) => this.onDown(e));
        overlay.addEventListener('pointermove', (e) => this.onMove(e));
        overlay.addEventListener('pointerup', (e) => this.onUp(e));
        overlay.addEventListener('pointercancel', (e) => this.onUp(e));
        overlay.addEventListener('pointerleave', () => {
            this.cursor = null;
            this.request();
        });
        overlay.addEventListener('wheel', (e) => this.onWheel(e), { passive: false });
        overlay.addEventListener('auxclick', (e) => e.preventDefault());
        new ResizeObserver(() => this.resize()).observe(stage);
        this.resize();
    }

    resize() {
        this.view.resize();
        const dpr = window.devicePixelRatio || 1;
        this.overlay.width = Math.round(this.view.cssW * dpr);
        this.overlay.height = Math.round(this.view.cssH * dpr);
        this.redraw();
    }

    /** Coalesces redraws into one per animation frame. */
    request() {
        if (this.frame) return;
        this.frame = requestAnimationFrame(() => {
            this.frame = 0;
            this.redraw();
        });
    }

    redraw() {
        this.view.draw();
        this.drawOverlay();
    }

    fit() {
        this.view.fit();
        this.request();
    }

    setSpace(down) {
        this.space = down;
        this.stage.classList.toggle('can-pan', down && !this.pan);
    }

    capture(e) {
        try {
            this.overlay.setPointerCapture(e.pointerId);
        } catch {
            // Synthetic or already released pointers cannot be captured; dragging still works.
        }
    }

    local(e) {
        const r = this.overlay.getBoundingClientRect();
        return [e.clientX - r.left, e.clientY - r.top];
    }

    /** Handle under a screen point: active quad first, then the others from the top. */
    hitHandle(s) {
        const order = state.quads.map((_, i) => i).filter((i) => i !== state.active);
        if (state.active >= 0) order.push(state.active);
        for (let k = order.length - 1; k >= 0; k--) {
            const q = order[k];
            const corners = state.quads[q].corners;
            for (let c = 0; c < 4; c++) {
                const p = this.view.toScreen(corners[c]);
                if (Math.hypot(p[0] - s[0], p[1] - s[1]) <= HIT_R) return { q, c };
            }
        }
        return null;
    }

    hitQuad(imgPt) {
        for (let q = state.quads.length - 1; q >= 0; q--) {
            if (pointInPolygon(imgPt, state.quads[q].corners)) return q;
        }
        return -1;
    }

    onDown(e) {
        const s = this.local(e);
        this.cursor = s;
        if (e.button === 1 || (e.button === 0 && this.space)) {
            e.preventDefault();
            this.pan = { x: s[0], y: s[1] };
            this.capture(e);
            this.stage.classList.add('is-panning');
            return;
        }
        if (e.button !== 0 || !state.source) return;
        const img = this.view.toImage(s[0], s[1]);
        if (state.draft.length > 0) {
            this.addDraftPoint(img);
            return;
        }
        const h = this.hitHandle(s);
        if (h) {
            setActive(h.q);
            this.drag = h;
            this.capture(e);
            this.request();
            return;
        }
        // A click inside an inactive quad activates it; anywhere else starts a new quad.
        const q = this.hitQuad(img);
        if (q >= 0 && q !== state.active) {
            setActive(q);
            this.request();
            return;
        }
        this.addDraftPoint(img);
    }

    addDraftPoint(img) {
        state.draft.push(img);
        if (state.draft.length === 4) {
            const corners = state.draft;
            state.draft = [];
            addQuad(corners);
        } else {
            emit('draft');
        }
        this.request();
    }

    cancelDraft() {
        if (state.draft.length === 0) return false;
        state.draft = [];
        emit('draft');
        this.request();
        return true;
    }

    onMove(e) {
        const s = this.local(e);
        this.cursor = s;
        if (this.pan) {
            this.view.panBy(s[0] - this.pan.x, s[1] - this.pan.y);
            this.pan = { x: s[0], y: s[1] };
            this.request();
            return;
        }
        if (this.drag) {
            moveCorner(this.drag.q, this.drag.c, this.view.toImage(s[0], s[1]));
            this.request();
            return;
        }
        const h = state.draft.length === 0 ? this.hitHandle(s) : null;
        const changed = (h?.q ?? -1) !== (this.hover?.q ?? -1) || (h?.c ?? -1) !== (this.hover?.c ?? -1);
        this.hover = h;
        this.stage.classList.toggle('over-handle', Boolean(h));
        if (changed || state.draft.length > 0) this.request();
    }

    onUp(e) {
        if (this.pan) {
            this.pan = null;
            this.stage.classList.remove('is-panning');
            this.stage.classList.toggle('can-pan', this.space);
        }
        if (this.drag) {
            this.drag = null;
            emit('quads'); // final geometry for list and preview
        }
        if (this.overlay.hasPointerCapture?.(e.pointerId)) this.overlay.releasePointerCapture(e.pointerId);
    }

    onWheel(e) {
        e.preventDefault();
        if (!state.source) return;
        const s = this.local(e);
        const unit = e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? 400 : 1;
        this.view.zoomAt(s[0], s[1], Math.pow(1.0015, -e.deltaY * unit));
        this.request();
    }

    drawOverlay() {
        const ctx = this.ctx;
        const dpr = window.devicePixelRatio || 1;
        ctx.setTransform(1, 0, 0, 1, 0, 0);
        ctx.clearRect(0, 0, this.overlay.width, this.overlay.height);
        if (!state.source) return;
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
        ctx.lineJoin = 'round';

        state.quads.forEach((q, i) => {
            if (i !== state.active) this.drawQuad(q, false);
        });
        const a = activeQuad();
        if (a) this.drawQuad(a, true);
        this.drawDraft();
    }

    path(points) {
        const ctx = this.ctx;
        ctx.beginPath();
        points.forEach((p, i) => {
            const [x, y] = this.view.toScreen(p);
            if (i === 0) ctx.moveTo(x, y);
            else ctx.lineTo(x, y);
        });
    }

    stroke(color, dash = []) {
        const ctx = this.ctx;
        ctx.setLineDash([]);
        ctx.strokeStyle = C.halo;
        ctx.lineWidth = 3;
        ctx.stroke();
        ctx.setLineDash(dash);
        ctx.strokeStyle = color;
        ctx.lineWidth = 1;
        ctx.stroke();
        ctx.setLineDash([]);
    }

    drawQuad(q, active) {
        const info = quadInfo(q);
        const color = info.invalid ? C.danger : active ? C.blue : C.muted;
        this.path(q.corners);
        this.ctx.closePath();
        this.stroke(color);
        const qi = state.quads.indexOf(q);
        q.corners.forEach((p, c) => {
            const hovered = this.hover && this.hover.q === qi && this.hover.c === c;
            this.handle(p, active, hovered, c === 0 ? color : null);
        });
    }

    /** Corner handle; `ring` marks the first corner, which becomes the texture's top-left. */
    handle(p, active, hovered, ring) {
        const ctx = this.ctx;
        const [x, y] = this.view.toScreen(p);
        const r = (active ? HANDLE_R : HANDLE_R_IDLE) + (hovered ? 1 : 0);
        if (ring) {
            ctx.beginPath();
            ctx.arc(x, y, r + 2.5, 0, Math.PI * 2);
            ctx.strokeStyle = C.halo;
            ctx.lineWidth = 3;
            ctx.stroke();
            ctx.strokeStyle = ring;
            ctx.lineWidth = 1.5;
            ctx.stroke();
        }
        ctx.beginPath();
        ctx.arc(x, y, r, 0, Math.PI * 2);
        ctx.fillStyle = active ? C.thumb : C.muted;
        ctx.fill();
        ctx.strokeStyle = C.thumbEdge;
        ctx.lineWidth = 1;
        ctx.stroke();
    }

    drawDraft() {
        const d = state.draft;
        if (d.length === 0) return;
        this.path(d);
        this.stroke(C.blue);
        if (this.cursor) {
            const last = this.view.toScreen(d[d.length - 1]);
            const ctx = this.ctx;
            ctx.beginPath();
            ctx.moveTo(last[0], last[1]);
            ctx.lineTo(this.cursor[0], this.cursor[1]);
            if (d.length === 3) {
                const first = this.view.toScreen(d[0]);
                ctx.lineTo(first[0], first[1]);
            }
            this.stroke(C.blue, [4, 3]);
        }
        d.forEach((p, i) => this.handle(p, true, false, i === 0 ? C.blue : null));
    }
}
