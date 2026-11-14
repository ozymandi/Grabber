// Canvas interaction (spec quad-editing, quad-shapes): zoom/pan, creating quads by four
// clicks, dragging the active quad's corners, activating quads by their shape, and editing a
// Path with Pen-tool gestures (design D5-D7 of add-quad-shapes). Draws on a 2D overlay above
// the WebGL view: every shape as marching ants, the quad itself only for the active one.

import { strokeAnts } from './ants.js';
import { record } from './history.js';
import { PEN_CURSORS } from './icons.js';
import * as shapes from './shape.js';
import { activeQuad, addQuad, emit, moveCorner, onChange, quadInfo, setActive, setNodes, state } from './state.js';

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
const HIT_R = 8; // grab distance for handles, anchors and segments, CSS px
const CLICK = 3; // a press that moves less than this is a click, CSS px
const ANCHOR = 6; // path anchor square, CSS px
const KNOB_R = 3; // path handle knob radius, CSS px

function pointInPolygon(p, poly) {
    let inside = false;
    for (let i = 0, j = poly.length - 1; i < poly.length; j = i++) {
        const [xi, yi] = poly[i];
        const [xj, yj] = poly[j];
        if (yi > p[1] !== yj > p[1] && p[0] < ((xj - xi) * (p[1] - yi)) / (yj - yi) + xi) inside = !inside;
    }
    return inside;
}

const dist = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1]);
const clamp01 = (v) => Math.min(Math.max(v, 0), 1);

/** Distance from p to segment ab and the parameter of the nearest point. */
function segDist(p, a, b) {
    const dx = b[0] - a[0];
    const dy = b[1] - a[1];
    const l2 = dx * dx + dy * dy;
    const t = l2 > 0 ? Math.min(Math.max(((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2, 0), 1) : 0;
    return { d: Math.hypot(p[0] - a[0] - t * dx, p[1] - a[1] - t * dy), t };
}

export class Editor {
    constructor(stage, view, overlay) {
        this.stage = stage;
        this.view = view;
        this.overlay = overlay;
        this.ctx = overlay.getContext('2d');
        this.cursor = null; // last pointer position, CSS px
        this.hover = null; // what lies under the pointer, for the cursor and handle highlight
        this.drag = null; // the gesture in progress, see onDown
        this.pan = null; // { x, y } last pan pointer position
        this.pen = null; // P drawing: { q, nodes, down }
        this.space = false;
        this.mods = { alt: false, ctrl: false }; // held modifiers, for the Pen cursor
        this.penCursor = '';
        this.frame = 0;
        this.overlayFrame = 0;

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
        // Pen drawing belongs to one Path quad; switching quads, shapes or images ends it.
        onChange((what) => {
            if (this.pen && (what === 'source' || !this.penQuadOk())) this.cancelPen();
            this.updateCursor();
        });
        // Ctrl and Alt change the cursor at once, without moving the mouse (as in Illustrator).
        const mods = (e) => {
            const next = { alt: e.altKey, ctrl: e.ctrlKey || e.metaKey };
            if (next.alt === this.mods.alt && next.ctrl === this.mods.ctrl) return;
            this.mods = next;
            this.updateCursor();
        };
        window.addEventListener('keydown', mods);
        window.addEventListener('keyup', mods);
        window.addEventListener('blur', () => mods({ altKey: false, ctrlKey: false, metaKey: false }));
        this.resize();
    }

    resize() {
        this.view.resize();
        const dpr = window.devicePixelRatio || 1;
        this.overlay.width = Math.round(this.view.cssW * dpr);
        this.overlay.height = Math.round(this.view.cssH * dpr);
        this.redraw();
    }

    /** Coalesces full redraws (image and overlay) into one per animation frame. */
    request() {
        if (this.frame) return;
        this.frame = requestAnimationFrame(() => {
            this.frame = 0;
            this.redraw();
        });
    }

    /** Redraws only the overlay on the next frame: the marching ants (design D5). */
    requestOverlay() {
        if (this.overlayFrame || this.frame) return;
        this.overlayFrame = requestAnimationFrame(() => {
            this.overlayFrame = 0;
            this.drawOverlay();
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

    // --- Geometry of quads on screen ---

    /** Screen geometry of quad `qi`: clockwise corners, output size, the unit -> screen map. */
    geo(qi) {
        const q = state.quads[qi];
        const info = quadInfo(q);
        const toScreen = (u) => {
            const p = shapes.unitToImage(info.cw, u);
            return p ? this.view.toScreen(p) : null;
        };
        return { q, info, toScreen };
    }

    /** The shape's outline on screen, flattened to about a quarter of a CSS pixel. */
    screenOutline(qi) {
        const { q, info, toScreen } = this.geo(qi);
        const sc = info.cw.map((p) => this.view.toScreen(p));
        const span = Math.max(dist(sc[0], sc[1]), dist(sc[1], sc[2]), dist(sc[2], sc[3]), dist(sc[3], sc[0]), 1);
        const unit = shapes.outlineTol(q.shape, 1, 1, Math.min(0.25 / span, 0.05));
        return unit.map(toScreen).filter(Boolean);
    }

    /** Unit-square point of a screen point on quad `qi`, or null beyond its horizon. */
    toUnit(qi, s) {
        return shapes.imageToUnit(quadInfo(state.quads[qi]).cw, this.view.toImage(s[0], s[1]));
    }

    /** Active quad's corner under a screen point. */
    hitCorner(s) {
        const q = activeQuad();
        if (!q) return null;
        for (let c = 0; c < 4; c++) {
            if (dist(this.view.toScreen(q.corners[c]), s) <= HIT_R) return { q: state.active, c };
        }
        return null;
    }

    /** Topmost quad whose shape contains the screen point. */
    hitShape(s) {
        for (let q = state.quads.length - 1; q >= 0; q--) {
            if (pointInPolygon(s, this.screenOutline(q))) return q;
        }
        return -1;
    }

    /** Path parts of the active quad under a screen point: a handle, an anchor or a segment. */
    hitPath(s) {
        const q = activeQuad();
        if (!q || q.shape.kind !== 'path') return null;
        const qi = state.active;
        const { toScreen } = this.geo(qi);
        const nodes = q.shape.nodes;
        for (let i = 0; i < nodes.length; i++) {
            for (const which of ['hin', 'hout']) {
                const h = nodes[i][which];
                const p = h && toScreen(h);
                if (p && dist(p, s) <= HIT_R) return { kind: 'handle', q: qi, i, which };
            }
        }
        for (let i = 0; i < nodes.length; i++) {
            const p = toScreen(nodes[i].p);
            if (p && dist(p, s) <= HIT_R) return { kind: 'anchor', q: qi, i };
        }
        let best = null;
        for (let i = 0; i < nodes.length; i++) {
            const c = shapes.segment(nodes, i);
            const N = 48;
            let prev = toScreen(c[0]);
            for (let k = 1; k <= N; k++) {
                const cur = toScreen(shapes.cubicAt(c, k / N));
                if (prev && cur) {
                    const r = segDist(s, prev, cur);
                    if (r.d <= HIT_R && (!best || r.d < best.d)) best = { d: r.d, i, t: (k - 1 + r.t) / N };
                }
                prev = cur;
            }
        }
        return best && { kind: 'segment', q: qi, i: best.i, t: best.t };
    }

    // --- Pointer ---

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
        if (this.pen) {
            this.penDown(e, s);
            return;
        }
        const alt = e.altKey;
        const ctrl = e.ctrlKey || e.metaKey;
        const hit = this.hitPath(s);
        if (hit) {
            e.preventDefault();
            const node = state.quads[hit.q].shape.nodes[hit.i];
            const start = { s0: s, moved: false, q: hit.q, i: hit.i };
            if (hit.kind === 'handle') {
                // Ctrl or plain: move the handle (mirrored when smooth); Alt: break it.
                this.drag = { ...start, kind: 'handle', which: hit.which, broken: alt };
            } else if (hit.kind === 'anchor') {
                if (alt) this.drag = { ...start, kind: 'pull', node: shapes.cloneNode(node) };
                else if (ctrl) this.drag = { ...start, kind: 'anchor', node: shapes.cloneNode(node), u0: this.toUnit(hit.q, s) };
                else this.drag = { ...start, kind: 'anchor-click', corner: this.hitCorner(s) };
            } else {
                this.drag = { ...start, kind: 'segment-click', t: hit.t };
            }
            this.capture(e);
            return;
        }
        const h = this.hitCorner(s);
        if (h) {
            this.drag = { kind: 'corner', ...h, recorded: false };
            this.capture(e);
            this.request();
            return;
        }
        // A click inside an inactive quad's shape activates it; anywhere else starts a new quad.
        const q = this.hitShape(s);
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
        if (this.pen) {
            this.penMove(e, s);
            return;
        }
        const d = this.drag;
        if (d) {
            if (!d.moved && dist(s, d.s0 ?? s) < CLICK && d.kind !== 'corner') return;
            d.moved = true;
            this.dragTo(d, s, e.shiftKey);
            this.request();
            return;
        }
        this.updateHover(s);
    }

    updateHover(s) {
        const h = state.draft.length === 0 && !this.pen ? (this.hitPath(s) ?? this.hitCorner(s)) : null;
        const key = h ? `${h.kind ?? 'corner'}:${h.q}:${h.i ?? h.c}:${h.which ?? ''}` : '';
        const changed = key !== (this.hover?.key ?? '');
        this.hover = h ? { ...h, key } : null;
        this.stage.classList.toggle('over-handle', Boolean(h && !h.kind));
        this.updateCursor();
        if (changed || state.draft.length > 0) this.request();
    }

    /**
     * The Pen cursor (spec "Pen cursors"): what a click or drag would do here. Over a path
     * segment, add; over an anchor, remove (Alt converts, Ctrl moves); over a handle, move
     * (Alt converts); while drawing with P, a new anchor or, over the first one, closing.
     * During a drag the cursor of the gesture stays.
     */
    updateCursor() {
        if (this.drag) return;
        let kind = '';
        if (this.pen && state.draft.length === 0) {
            const p = this.pen;
            const first = p.nodes.length >= 2 && this.cursor && this.geo(p.q).toScreen(p.nodes[0].p);
            kind = first && dist(first, this.cursor) <= HIT_R ? 'pen-close' : 'pen';
        } else if (this.hover?.kind && state.draft.length === 0) {
            const { alt, ctrl } = this.mods;
            const k = this.hover.kind;
            if (k === 'segment') kind = 'pen-add';
            else if (alt) kind = 'convert';
            else if (ctrl || k === 'handle') kind = 'direct';
            else kind = 'pen-remove';
        }
        if (kind === this.penCursor) return;
        this.penCursor = kind;
        this.stage.classList.toggle('pen-cursor', Boolean(kind));
        if (kind) this.stage.style.setProperty('--pen-cursor', PEN_CURSORS[kind]);
    }

    /** One step of a drag. Each gesture records its undo step on its first movement. */
    dragTo(d, s, shift) {
        const once = () => {
            if (!d.recorded) {
                record();
                d.recorded = true;
            }
        };
        if (d.kind === 'corner') {
            once();
            moveCorner(d.q, d.c, this.view.toImage(s[0], s[1]));
            return;
        }
        if (d.kind === 'anchor-click') {
            // A plain drag from an anchor lying on a quad corner moves the corner (design D6).
            if (d.corner) {
                Object.assign(d, { kind: 'corner', q: d.corner.q, c: d.corner.c });
                this.dragTo(d, s, shift);
            }
            return;
        }
        if (d.kind === 'segment-click') return;
        const u = this.toUnit(d.q, s);
        if (!u) return;
        const q = state.quads[d.q];
        const info = quadInfo(q);
        const nodes = q.shape.nodes.map(shapes.cloneNode);
        const n = nodes[d.i];
        // Shift: 45-degree steps of `v - from`, measured in output pixels (the quad's plane).
        const snap45 = (from, v) => {
            if (!shift) return v;
            const dx = (v[0] - from[0]) * info.w;
            const dy = (v[1] - from[1]) * info.h;
            const a = Math.round(Math.atan2(dy, dx) / (Math.PI / 4)) * (Math.PI / 4);
            const r = Math.hypot(dx, dy);
            return [from[0] + (Math.cos(a) * r) / info.w, from[1] + (Math.sin(a) * r) / info.h];
        };
        once();
        if (d.kind === 'anchor') {
            const to = snap45(d.u0, u);
            let dx = to[0] - d.u0[0];
            let dy = to[1] - d.u0[1];
            dx = clamp01(d.node.p[0] + dx) - d.node.p[0];
            dy = clamp01(d.node.p[1] + dy) - d.node.p[1];
            const mv = (p) => p && [p[0] + dx, p[1] + dy];
            Object.assign(n, { p: mv(d.node.p), hin: mv(d.node.hin), hout: mv(d.node.hout) });
        } else if (d.kind === 'pull') {
            // Alt+drag from an anchor: symmetric handles, the anchor becomes smooth.
            const h = snap45(d.node.p, u);
            n.hout = h;
            n.hin = [2 * n.p[0] - h[0], 2 * n.p[1] - h[1]];
            n.smooth = true;
        } else if (d.kind === 'handle') {
            const h = snap45(n.p, u);
            n[d.which] = h;
            const other = d.which === 'hin' ? 'hout' : 'hin';
            if (d.broken) {
                n.smooth = false;
            } else if (n.smooth && n[other]) {
                // Keep the partner on the opposite side, at its own length (in output pixels).
                const vx = (h[0] - n.p[0]) * info.w;
                const vy = (h[1] - n.p[1]) * info.h;
                const len = Math.hypot(vx, vy);
                const olen = Math.hypot((n[other][0] - n.p[0]) * info.w, (n[other][1] - n.p[1]) * info.h);
                if (len > 0) n[other] = [n.p[0] - ((vx / len) * olen) / info.w, n.p[1] - ((vy / len) * olen) / info.h];
            }
        }
        setNodes(d.q, nodes);
    }

    onUp(e) {
        if (this.pan) {
            this.pan = null;
            this.stage.classList.remove('is-panning');
            this.stage.classList.toggle('can-pan', this.space);
        }
        if (this.pen) {
            this.penUp(e);
        } else if (this.drag) {
            const d = this.drag;
            this.drag = null;
            if (!d.moved) this.click(d, e);
            emit('quads'); // final geometry for list and preview
        }
        if (this.overlay.hasPointerCapture?.(e.pointerId)) this.overlay.releasePointerCapture(e.pointerId);
        this.updateHover(this.local(e)); // the cursor for what now lies under the pointer
    }

    /** A press on the path that did not move: remove, add or convert an anchor. */
    click(d) {
        const q = state.quads[d.q];
        if (!q || q.shape.kind !== 'path') return;
        const nodes = q.shape.nodes;
        if (d.kind === 'anchor-click') {
            if (nodes.length <= 2) return; // a path keeps at least 2 anchors
            record();
            setNodes(d.q, nodes.filter((_, i) => i !== d.i).map(shapes.cloneNode));
        } else if (d.kind === 'segment-click') {
            record();
            setNodes(d.q, shapes.split(nodes, d.i, d.t));
        } else if (d.kind === 'pull') {
            // Alt+click on a smooth anchor makes it a corner.
            const n = nodes[d.i];
            if (!n.hin && !n.hout) return;
            record();
            const next = nodes.map(shapes.cloneNode);
            Object.assign(next[d.i], { hin: null, hout: null, smooth: false });
            setNodes(d.q, next);
        }
    }

    onWheel(e) {
        e.preventDefault();
        if (!state.source) return;
        const s = this.local(e);
        const unit = e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? 400 : 1;
        this.view.zoomAt(s[0], s[1], Math.pow(1.0015, -e.deltaY * unit));
        this.request();
    }

    // --- Pen drawing (P, design D7) ---

    penQuadOk() {
        return this.pen && state.active === this.pen.q && activeQuad()?.shape.kind === 'path';
    }

    /** Starts drawing a new path for the active Path quad. Returns false when it cannot. */
    startPen() {
        const q = activeQuad();
        if (!q || q.shape.kind !== 'path' || state.draft.length) return false;
        this.drag = null;
        this.pen = { q: state.active, nodes: [], down: null };
        this.stage.classList.add('is-pen');
        this.updateCursor();
        this.request();
        return true;
    }

    /** Esc while drawing: the previous path stays. */
    cancelPen() {
        if (!this.pen) return false;
        this.pen = null;
        this.stage.classList.remove('is-pen');
        this.updateCursor();
        this.request();
        return true;
    }

    penDown(e, s) {
        const p = this.pen;
        const u = this.toUnit(p.q, s);
        if (!u) return;
        const { toScreen } = this.geo(p.q);
        const first = p.nodes.length >= 2 && toScreen(p.nodes[0].p);
        if (first && dist(first, s) <= HIT_R) {
            p.down = { close: true, s0: s, moved: false };
        } else {
            p.nodes.push(shapes.corner([clamp01(u[0]), clamp01(u[1])]));
            p.down = { close: false, s0: s, moved: false };
        }
        this.capture(e);
        this.request();
    }

    penMove(e, s) {
        const p = this.pen;
        if (p.down && (p.down.moved || dist(s, p.down.s0) >= CLICK)) {
            p.down.moved = true;
            const u = this.toUnit(p.q, s);
            if (u) {
                // Click-drag makes a smooth anchor: the outgoing handle follows the pointer.
                // On the closing click the first anchor's handles are reshaped the same way.
                const n = p.down.close ? p.nodes[0] : p.nodes[p.nodes.length - 1];
                n.hout = u;
                n.hin = [2 * n.p[0] - u[0], 2 * n.p[1] - u[1]];
                n.smooth = true;
            }
        }
        this.updateCursor();
        this.request();
    }

    penUp() {
        const p = this.pen;
        if (!p.down) return;
        const close = p.down.close;
        p.down = null;
        if (close) {
            const q = state.quads[p.q];
            if (p.nodes.length >= 2 && q?.shape.kind === 'path') {
                record();
                this.pen = null;
                this.stage.classList.remove('is-pen');
                setNodes(p.q, p.nodes);
                emit('quads');
            }
        }
        this.request();
    }

    // --- Drawing ---

    drawOverlay() {
        const ctx = this.ctx;
        const dpr = window.devicePixelRatio || 1;
        ctx.setTransform(1, 0, 0, 1, 0, 0);
        ctx.clearRect(0, 0, this.overlay.width, this.overlay.height);
        if (!state.source) return;
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
        ctx.lineJoin = 'round';

        const a = activeQuad();
        if (a) this.drawQuadFrame(a);
        state.quads.forEach((_, i) => this.drawAnts(i));
        if (a) {
            this.drawCorners(a);
            if (a.shape.kind === 'path' && !this.pen) this.drawPathHandles(state.active);
        }
        if (this.pen) this.drawPen();
        this.drawDraft();
        // The ants keep marching while there are shapes to show (overlay only, design D5).
        if (state.quads.length && this.overlay.offsetWidth > 0) this.requestOverlay();
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

    /** The active quad's edges (blue, or danger when it cannot be exported). */
    drawQuadFrame(q) {
        const info = quadInfo(q);
        this.path(q.corners);
        this.ctx.closePath();
        this.stroke(info.invalid ? C.danger : C.blue);
    }

    drawAnts(qi) {
        const pts = this.screenOutline(qi);
        if (pts.length < 2) return;
        const ctx = this.ctx;
        ctx.beginPath();
        pts.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
        ctx.closePath();
        strokeAnts(ctx, 1);
    }

    drawCorners(q) {
        const info = quadInfo(q);
        const color = info.invalid ? C.danger : C.blue;
        q.corners.forEach((p, c) => {
            const hovered = this.hover && !this.hover.kind && this.hover.c === c;
            this.handle(p, hovered, c === 0 ? color : null);
        });
    }

    /** Corner handle; `ring` marks the first corner, which becomes the texture's top-left. */
    handle(p, hovered, ring) {
        const ctx = this.ctx;
        const [x, y] = this.view.toScreen(p);
        const r = HANDLE_R + (hovered ? 1 : 0);
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
        ctx.fillStyle = C.thumb;
        ctx.fill();
        ctx.strokeStyle = C.thumbEdge;
        ctx.lineWidth = 1;
        ctx.stroke();
    }

    /** Anchors as squares and handles as knobs on lines, like Illustrator (design D5). */
    drawPathHandles(qi, nodes = state.quads[qi].shape.nodes) {
        const { toScreen } = this.geo(qi);
        const ctx = this.ctx;
        const hov = this.hover?.kind && this.hover.q === qi ? this.hover : null;
        nodes.forEach((n, i) => {
            const p = toScreen(n.p);
            if (!p) return;
            for (const which of ['hin', 'hout']) {
                const h = n[which] && toScreen(n[which]);
                if (!h) continue;
                ctx.beginPath();
                ctx.moveTo(p[0], p[1]);
                ctx.lineTo(h[0], h[1]);
                this.stroke(C.blue);
                ctx.beginPath();
                ctx.arc(h[0], h[1], KNOB_R + (hov?.kind === 'handle' && hov.i === i && hov.which === which ? 1 : 0), 0, Math.PI * 2);
                ctx.fillStyle = C.blue;
                ctx.fill();
                ctx.strokeStyle = C.halo;
                ctx.lineWidth = 1;
                ctx.stroke();
            }
            const a = ANCHOR + (hov?.kind === 'anchor' && hov.i === i ? 2 : 0);
            ctx.beginPath();
            ctx.rect(Math.round(p[0] - a / 2) + 0.5, Math.round(p[1] - a / 2) + 0.5, a - 1, a - 1);
            ctx.fillStyle = n.smooth || n.hin || n.hout ? C.blue : C.thumb;
            ctx.fill();
            ctx.strokeStyle = C.blue;
            ctx.lineWidth = 1;
            ctx.stroke();
        });
    }

    /** The path being drawn with the Pen, with a rubber band to the pointer. */
    drawPen() {
        const p = this.pen;
        const { toScreen } = this.geo(p.q);
        const ctx = this.ctx;
        const nodes = p.nodes;
        if (nodes.length) {
            ctx.beginPath();
            const s0 = toScreen(nodes[0].p);
            if (s0) ctx.moveTo(s0[0], s0[1]);
            for (let i = 0; i + 1 < nodes.length; i++) {
                const c = [nodes[i].p, nodes[i].hout ?? nodes[i].p, nodes[i + 1].hin ?? nodes[i + 1].p, nodes[i + 1].p];
                for (let k = 1; k <= 32; k++) {
                    const s = toScreen(shapes.cubicAt(c, k / 32));
                    if (s) ctx.lineTo(s[0], s[1]);
                }
            }
            if (this.cursor && !p.down) {
                const last = toScreen(nodes[nodes.length - 1].p);
                if (last) ctx.moveTo(last[0], last[1]);
                ctx.lineTo(this.cursor[0], this.cursor[1]);
            }
            this.stroke(C.blue);
            this.drawPathHandles(p.q, nodes);
        }
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
        d.forEach((p, i) => this.handle(p, false, i === 0 ? C.blue : null));
    }
}
