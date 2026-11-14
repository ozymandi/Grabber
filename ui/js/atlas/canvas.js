// ATLAS canvas (design D6 of add-texture-atlas, D5 of add-atlas-fine-work): sectors with their
// graph-paper grid and UV guides, tiles at any angle and the "+" buttons on one 2D canvas with
// pan and zoom. It never changes the workspace itself: every committed edit goes through
// `hooks.edit`, and the canvas redraws from the document Rust returns.
// Canvas space matches Rust: x right, y down; sector (u, v) spans y in [-(v+1)S, -vS).

import { PLUS_ICON, ROTATE_CURSOR, CROSSHAIR } from '../icons.js';
import * as F from './frame.js';
import * as Snap from './snap.js';
import * as Marquee from './marquee.js';
import { strokeAnts } from '../ants.js';

const HANDLE = 6; // half size of a corner handle, screen px
const ROTATE_ZONE = 16; // depth of the rotate zone outside a corner, CSS px
const PIVOT_HIT = 10; // pivot grab radius, CSS px
const PIVOT_SNAP = 8; // pivot snap distance, CSS px
const PIVOT_SIZE = 16; // pivot marker size, CSS px
const MAGENTA = '#FF00FF'; // guide preview colour (named deviation: not an O'Tool token)
const SIDES = [
    { side: 'left', du: -1, dv: 0 },
    { side: 'right', du: 1, dv: 0 },
    { side: 'up', du: 0, dv: 1 },
    { side: 'down', du: 0, dv: -1 },
];

const css = (name) => getComputedStyle(document.documentElement).getPropertyValue(name).trim();

export const sectorRect = (s, size) => ({ x: s.u * size, y: -(s.v + 1) * size, w: size, h: size });
const inRect = (p, r) => p[0] >= r.x && p[0] < r.x + r.w && p[1] >= r.y && p[1] < r.y + r.h;
const sameSector = (a, b) => a.u === b.u && a.v === b.v;
const dist = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1]);
const norm360 = (a) => ((a % 360) + 360) % 360;
const deg = (v) => (Math.atan2(v[1], v[0]) * 180) / Math.PI;

export class AtlasCanvas {
    /**
     * `hooks.edit(edit)` commits an edit; `hooks.select()` reports selection changes;
     * `hooks.status(text)` shows live feedback (size while scaling, angle while rotating).
     */
    constructor(stage, canvas, plusLayer, hooks) {
        this.stage = stage;
        this.canvas = canvas;
        this.ctx = canvas.getContext('2d');
        this.plusLayer = plusLayer;
        this.hooks = hooks;
        this.doc = null;
        this.images = new Map(); // item -> ImageBitmap | 'loading'
        this.tinted = new Map(); // guide item -> magenta canvas, built while the preview is on
        this.selTiles = new Set();
        this.selSectors = [];
        this.k = 0.25;
        this.ox = 0;
        this.oy = 0;
        this.fitted = false;
        this.drag = null; // { kind: 'move'|'scale'|'rotate'|'pivot'|'pan', ... }
        this.space = false;
        this.preview = false;
        this.opacity = null; // live Opacity while its slider is dragged
        this.pending = false;
        this.colors = null;
        stage.style.setProperty('--rotate-cursor', ROTATE_CURSOR);
        new ResizeObserver(() => this.resize()).observe(stage);
        canvas.addEventListener('pointerdown', (e) => this.onDown(e));
        window.addEventListener('pointermove', (e) => this.onMove(e));
        window.addEventListener('pointerup', (e) => this.onUp(e));
        canvas.addEventListener('dblclick', (e) => this.onDblClick(e));
        canvas.addEventListener('wheel', (e) => this.onWheel(e), { passive: false });
        window.addEventListener('keydown', (e) => this.onCtrl(e, true));
        window.addEventListener('keyup', (e) => this.onCtrl(e, false));
    }

    get busy() {
        return this.drag !== null;
    }

    resize() {
        const r = this.stage.getBoundingClientRect();
        const dpr = window.devicePixelRatio || 1;
        this.canvas.width = Math.max(1, Math.round(r.width * dpr));
        this.canvas.height = Math.max(1, Math.round(r.height * dpr));
        this.request();
    }

    setDoc(doc) {
        const first = !this.doc || this.doc.id !== doc.id;
        this.doc = doc;
        const ids = new Set(doc.manifest.tiles.map((t) => t.id));
        for (const id of [...this.selTiles]) if (!ids.has(id)) this.selTiles.delete(id);
        const sectors = doc.manifest.sectors;
        this.selSectors = this.selSectors.filter((s) => sectors.some((c) => sameSector(c, s)));
        if (first) {
            this.selTiles.clear();
            this.selSectors = [];
            this.fitted = false;
        }
        this.loadImages();
        if (!this.fitted && this.canvas.width > 1) this.fit();
        this.request();
    }

    loadImage(item, max) {
        if (this.images.has(item)) return;
        this.images.set(item, 'loading');
        fetch(`http://ripper.localhost/atlas/${this.doc.id}/${item}?max=${max}`)
            .then((r) => (r.ok ? r.blob() : Promise.reject(new Error(r.status))))
            .then((b) => createImageBitmap(b))
            .then((bmp) => {
                this.images.set(item, bmp);
                this.request();
            })
            .catch(() => this.images.delete(item));
    }

    loadImages() {
        for (const t of this.doc.manifest.tiles) this.loadImage(t.item, 2048);
        for (const g of this.doc.manifest.guides) this.loadImage(g.item, 4096);
    }

    /** Hold-G preview: guides drawn above the tiles in magenta. */
    setPreview(on) {
        if (this.preview === on) return;
        this.preview = on;
        if (!on) this.tinted.clear();
        this.request();
    }

    // ---- view ----

    toScreen(p) {
        return [p[0] * this.k + this.ox, p[1] * this.k + this.oy];
    }

    toCanvas(sx, sy) {
        return [(sx - this.ox) / this.k, (sy - this.oy) / this.k];
    }

    local(e) {
        const r = this.canvas.getBoundingClientRect();
        const dpr = window.devicePixelRatio || 1;
        return [(e.clientX - r.left) * dpr, (e.clientY - r.top) * dpr];
    }

    bounds() {
        const m = this.doc.manifest;
        const rs = m.sectors.map((s) => sectorRect(s, m.sector_size)).concat(m.tiles.map((t) => F.rect(t)));
        const x0 = Math.min(...rs.map((r) => r.x));
        const y0 = Math.min(...rs.map((r) => r.y));
        const x1 = Math.max(...rs.map((r) => r.x + r.w));
        const y1 = Math.max(...rs.map((r) => r.y + r.h));
        return { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
    }

    fit() {
        if (!this.doc) return;
        const b = this.bounds();
        const W = this.canvas.width;
        const H = this.canvas.height;
        const margin = 48 * (window.devicePixelRatio || 1);
        this.k = Math.min((W - 2 * margin) / b.w, (H - 2 * margin) / b.h);
        this.ox = (W - b.w * this.k) / 2 - b.x * this.k;
        this.oy = (H - b.h * this.k) / 2 - b.y * this.k;
        this.fitted = true;
        this.request();
    }

    zoomAt(sx, sy, f) {
        const k = Math.min(Math.max(this.k * f, 0.005), 16);
        const p = this.toCanvas(sx, sy);
        this.k = k;
        this.ox = sx - p[0] * k;
        this.oy = sy - p[1] * k;
        this.request();
    }

    setSpace(down) {
        this.space = down;
        this.stage.classList.toggle('can-pan', down);
    }

    // ---- hit testing ----

    /** Frame of a tile as shown, including an in-progress gesture. */
    shown(t) {
        const d = this.drag;
        if (!d) return t;
        if (d.kind === 'move' && this.selTiles.has(t.id)) return F.moved(t, d.dx, d.dy);
        if ((d.kind === 'scale' || d.kind === 'rotate' || d.kind === 'pivot') && d.id === t.id && d.frame) return d.frame;
        return t;
    }

    tileAt(p) {
        const tiles = this.doc.manifest.tiles;
        for (let i = tiles.length - 1; i >= 0; i--) {
            if (F.contains(this.shown(tiles[i]), p)) return tiles[i];
        }
        return null;
    }

    sectorAt(p) {
        const m = this.doc.manifest;
        return m.sectors.find((s) => inRect(p, sectorRect(s, m.sector_size))) ?? null;
    }

    /** The single selected tile, if exactly one is selected. */
    single() {
        if (this.selTiles.size !== 1) return null;
        return this.doc.manifest.tiles.find((x) => this.selTiles.has(x.id)) ?? null;
    }

    /** Handle under the screen point for the single selected tile: its corner name. */
    handleAt(s) {
        const t = this.single();
        if (!t) return null;
        const r = HANDLE * (window.devicePixelRatio || 1) + 2;
        const cs = F.corners(this.shown(t));
        for (let i = 0; i < 4; i++) {
            const p = this.toScreen(cs[i]);
            if (Math.abs(p[0] - s[0]) <= r && Math.abs(p[1] - s[1]) <= r) return { tile: t, corner: F.CORNERS[i] };
        }
        return null;
    }

    /** True when the screen point is in the rotate zone just outside a corner of the single tile. */
    rotateZoneAt(s) {
        const t = this.single();
        if (!t) return false;
        const f = this.shown(t);
        if (F.contains(f, this.toCanvas(...s))) return false;
        const zone = (ROTATE_ZONE + HANDLE) * (window.devicePixelRatio || 1);
        return F.corners(f).some((c) => dist(this.toScreen(c), s) <= zone);
    }

    pivotAt(s) {
        const t = this.single();
        if (!t) return false;
        return dist(this.toScreen(F.pivotCanvas(this.shown(t))), s) <= PIVOT_HIT * (window.devicePixelRatio || 1);
    }

    // ---- pointer ----

    onDown(e) {
        if (!this.doc) return;
        const s = this.local(e);
        if (e.button === 1 || (e.button === 0 && this.space)) {
            e.preventDefault();
            this.drag = { kind: 'pan', last: s };
            this.stage.classList.add('is-panning');
            return;
        }
        if (e.button !== 0) return;
        const p = this.toCanvas(...s);
        const h = this.handleAt(s);
        if (h) {
            this.drag = { kind: 'scale', id: h.tile.id, tile: h.tile, corner: h.corner, anchor: e.altKey ? 'pivot' : F.OPPOSITE[h.corner], frame: null };
            return;
        }
        const one = this.single();
        if (one && e.altKey && (this.pivotAt(s) || F.contains(one, p))) {
            e.preventDefault();
            this.drag = { kind: 'pivot', id: one.id, tile: one, start: s, frame: null, pivot: one.pivot };
            return;
        }
        if (this.rotateZoneAt(s)) {
            const pv = F.pivotCanvas(one);
            this.drag = { kind: 'rotate', id: one.id, tile: one, a0: deg([p[0] - pv[0], p[1] - pv[1]]), frame: null };
            return;
        }
        const t = this.tileAt(p);
        if (t) {
            if (e.shiftKey) {
                if (this.selTiles.has(t.id)) this.selTiles.delete(t.id);
                else this.selTiles.add(t.id);
            } else if (!this.selTiles.has(t.id)) {
                this.selTiles = new Set([t.id]);
            }
            this.hooks.select();
            if (this.selTiles.has(t.id)) {
                const m = this.doc.manifest;
                this.drag = {
                    kind: 'move',
                    start: p,
                    dx: 0,
                    dy: 0,
                    s0: s,
                    started: false,
                    last: s,
                    ctrl: e.ctrlKey,
                    box: Snap.boxOf(m.tiles.filter((x) => this.selTiles.has(x.id))),
                    targets: Snap.targets(m, this.selTiles),
                    lines: [],
                };
            }
            this.request();
            return;
        }
        // Empty canvas: a frame once the pointer moves 3 px, otherwise a click on release
        // (design D1 of add-atlas-marquee).
        this.drag = { kind: 'marquee', s0: s, p0: p, last: s, started: false, add: e.shiftKey, base: new Set(this.selTiles), sec: this.sectorAt(p), shift: e.shiftKey };
    }

    /** A click on empty canvas: selects or toggles the sector under it, or clears the selection. */
    clickEmpty(sec, shift) {
        if (sec) {
            if (shift) {
                const i = this.selSectors.findIndex((x) => sameSector(x, sec));
                if (i >= 0) this.selSectors.splice(i, 1);
                else this.selSectors.push(sec);
            } else {
                this.selSectors = [sec];
                this.selTiles.clear();
                this.hooks.edit({ op: 'set_active', sector: sec }, { quiet: true });
            }
        } else {
            this.selSectors = [];
            this.selTiles.clear();
        }
        this.hooks.select();
        this.request();
    }

    hover(s) {
        const h = this.handleAt(s);
        const nwse = h && (h.corner === 'top_left' || h.corner === 'bottom_right');
        const rot = !h && this.rotateZoneAt(s);
        this.stage.classList.toggle('over-handle-nwse', Boolean(h && nwse));
        this.stage.classList.toggle('over-handle-nesw', Boolean(h && !nwse));
        this.stage.classList.toggle('over-rotate', Boolean(rot));
        const t = !h && !rot && this.tileAt(this.toCanvas(...s));
        this.stage.classList.toggle('over-tile', Boolean(t));
    }

    onMove(e) {
        if (!this.doc) return;
        const s = this.local(e);
        const d = this.drag;
        if (!d) {
            if (e.target === this.canvas) this.hover(s);
            return;
        }
        const p = this.toCanvas(...s);
        if (d.kind === 'pan') {
            this.ox += s[0] - d.last[0];
            this.oy += s[1] - d.last[1];
            d.last = s;
        } else if (d.kind === 'move') {
            d.last = s;
            d.ctrl = e.ctrlKey;
            this.dragMove(d);
        } else if (d.kind === 'marquee') {
            d.last = s;
            this.dragMarquee(d);
        } else if (d.kind === 'scale') {
            this.dragScale(d, p);
        } else if (d.kind === 'rotate') {
            const t = d.tile;
            const pv = F.pivotCanvas(t);
            let total = F.angle(t) + (deg([p[0] - pv[0], p[1] - pv[1]]) - d.a0);
            if (e.shiftKey) total = Math.round(total / 15) * 15;
            d.angle = total;
            d.frame = F.turned(t, total);
            this.hooks.status(`Rotate ${Number(norm360(total).toFixed(1))}°`);
        } else if (d.kind === 'pivot') {
            if (!d.frame && dist(s, d.start) < 3) return;
            this.dragPivot(d, p, s);
        }
        this.request();
    }

    /** Move offset from the last pointer position, snapped unless Ctrl is held (design D1, D3). */
    dragMove(d) {
        // A click with a little jitter must not snap the tile away: start after 3 px.
        if (!d.started && dist(d.last, d.s0) < 3 * (window.devicePixelRatio || 1)) return;
        d.started = true;
        const p = this.toCanvas(...d.last);
        const rx = Math.round(p[0] - d.start[0]);
        const ry = Math.round(p[1] - d.start[1]);
        if (d.ctrl) {
            d.dx = rx;
            d.dy = ry;
            d.lines = [];
            return;
        }
        const r = Snap.snapMove(d.box, rx, ry, d.targets, this.doc.manifest.sector_size, this.k, window.devicePixelRatio || 1);
        d.dx = r.dx;
        d.dy = r.dy;
        d.lines = r.lines;
    }

    /** Selection from the frame, recomputed from the starting selection each time (design D2). */
    dragMarquee(d) {
        if (!d.started && dist(d.last, d.s0) < 3 * (window.devicePixelRatio || 1)) return;
        d.started = true;
        const box = Marquee.boxFrom(d.p0, this.toCanvas(...d.last));
        const sel = d.add ? new Set(d.base) : new Set();
        for (const t of this.doc.manifest.tiles) if (Marquee.touches(F.corners(t), box)) sel.add(t.id);
        this.selTiles = sel;
        this.hooks.select();
    }

    /** Esc during a frame drag: restores the selection it started from (design D4). */
    cancelMarquee() {
        const d = this.drag;
        if (!d || d.kind !== 'marquee') return false;
        this.drag = null;
        this.selTiles = d.base;
        this.hooks.select();
        this.request();
        return true;
    }

    /** Ctrl pressed or released during a move: recompute at once, without pointer movement. */
    onCtrl(e, down) {
        const d = this.drag;
        if (!d || d.kind !== 'move' || (e.code !== 'ControlLeft' && e.code !== 'ControlRight')) return;
        if (d.ctrl === down) return;
        d.ctrl = down;
        this.dragMove(d);
        this.request();
    }

    dragScale(d, p) {
        const t = d.tile;
        const l = F.toLocal(t, p);
        let w;
        if (d.anchor === 'pivot') {
            const P = F.pivotLocal(t);
            const C = F.cornerLocal(t, d.corner);
            const sx = Math.abs(C[0] - P[0]) > 0.5 ? Math.abs(l[0] - P[0]) / Math.abs(C[0] - P[0]) : 0;
            const sy = Math.abs(C[1] - P[1]) > 0.5 ? Math.abs(l[1] - P[1]) / Math.abs(C[1] - P[1]) : 0;
            const k = Math.max(sx, sy) || 1;
            w = Math.max(1, Math.round(t.w * k));
        } else {
            const A = F.cornerLocal(t, d.anchor);
            w = Math.max(1, Math.round(Math.max(Math.abs(l[0] - A[0]), (Math.abs(l[1] - A[1]) * t.w0) / t.h0)));
        }
        const h = Math.max(1, Math.round((w * t.h0) / t.w0));
        d.w = w;
        d.frame = F.scaled(t, w, h, d.anchor);
        this.hooks.status(`${w} × ${h}`);
    }

    dragPivot(d, p, s) {
        const t = d.tile;
        const l = F.toLocal(t, p);
        let q = [Math.round(l[0] + t.w / 2), Math.round(l[1] + t.h / 2)];
        let centre = false;
        const snap = PIVOT_SNAP * (window.devicePixelRatio || 1);
        for (const sp of F.snapPoints(t)) {
            const c = this.toScreen(F.toCanvas(t, [sp[0] - t.w / 2, sp[1] - t.h / 2]));
            if (dist(c, s) <= snap) {
                centre = sp[0] === t.w / 2 && sp[1] === t.h / 2;
                q = [Math.round(sp[0]), Math.round(sp[1])];
                break;
            }
        }
        d.pivot = centre ? null : q;
        d.frame = { ...t, pivot: d.pivot };
    }

    onUp() {
        const d = this.drag;
        if (!d) return;
        this.drag = null;
        this.stage.classList.remove('is-panning');
        const find = (id) => this.doc.manifest.tiles.find((x) => x.id === id);
        if (d.kind === 'marquee') {
            if (!d.started) this.clickEmpty(d.sec, d.shift);
        } else if (d.kind === 'move' && (d.dx !== 0 || d.dy !== 0)) {
            const ids = this.doc.manifest.tiles.filter((t) => this.selTiles.has(t.id)).map((t) => t.id);
            // Keep the moved position on screen until the new document arrives.
            for (const id of ids) Object.assign(find(id), F.moved(find(id), d.dx, d.dy));
            this.hooks.edit({ op: 'move', ids, dx: d.dx, dy: d.dy });
        } else if (d.kind === 'scale' && d.frame) {
            Object.assign(find(d.id) ?? {}, d.frame);
            this.hooks.edit({ op: 'scale', id: d.id, w: d.w, anchor: d.anchor });
        } else if (d.kind === 'rotate' && d.frame) {
            Object.assign(find(d.id) ?? {}, d.frame);
            this.hooks.edit({ op: 'turn', id: d.id, angle: d.angle });
        } else if (d.kind === 'pivot' && d.frame) {
            Object.assign(find(d.id) ?? {}, d.frame);
            this.hooks.edit({ op: 'set_pivot', id: d.id, pivot: d.pivot });
        }
        this.request();
    }

    onDblClick(e) {
        const t = this.single();
        if (!t || !e.altKey) return;
        const s = this.local(e);
        if (!this.pivotAt(s) && !F.contains(t, this.toCanvas(...s))) return;
        if (t.pivot) this.hooks.edit({ op: 'set_pivot', id: t.id, pivot: null });
    }

    onWheel(e) {
        e.preventDefault();
        const s = this.local(e);
        this.zoomAt(s[0], s[1], Math.exp(-e.deltaY * 0.0015));
    }

    // ---- drawing ----

    request() {
        if (this.pending) return;
        this.pending = true;
        requestAnimationFrame(() => {
            this.pending = false;
            this.draw();
        });
    }

    /** Screen rect of a sector, or null when it is off screen. */
    visible(r) {
        const [x0, y0] = this.toScreen([r.x, r.y]);
        const [x1, y1] = this.toScreen([r.x + r.w, r.y + r.h]);
        if (x1 < 0 || y1 < 0 || x0 > this.canvas.width || y0 > this.canvas.height) return null;
        return [x0, y0, x1, y1];
    }

    /** Graph-paper grid: major lines every S/8, minor every S/64 (hidden when too dense). */
    drawGrid(m, c, dpr) {
        const g = this.ctx;
        const S = m.sector_size;
        const showMinor = Snap.gridStep(S, this.k, dpr) === S / 64;
        const W = this.canvas.width;
        const H = this.canvas.height;
        const minor = new Path2D();
        const major = new Path2D();
        for (const s of m.sectors) {
            const v = this.visible(sectorRect(s, S));
            if (!v) continue;
            const [x0, y0, x1, y1] = v;
            const step = (x1 - x0) / 64;
            for (let i = 1; i < 64; i++) {
                const isMajor = i % 8 === 0;
                if (!isMajor && !showMinor) continue;
                const path = isMajor ? major : minor;
                const x = Math.round(x0 + i * step) + 0.5;
                if (x >= 0 && x <= W) {
                    path.moveTo(x, Math.max(y0, 0));
                    path.lineTo(x, Math.min(y1, H));
                }
                const y = Math.round(y0 + i * step) + 0.5;
                if (y >= 0 && y <= H) {
                    path.moveTo(Math.max(x0, 0), y);
                    path.lineTo(Math.min(x1, W), y);
                }
            }
        }
        g.lineWidth = dpr;
        g.strokeStyle = c.line;
        g.stroke(minor);
        g.strokeStyle = c.line2;
        g.stroke(major);
    }

    tint(item, img) {
        let t = this.tinted.get(item);
        if (t) return t;
        t = document.createElement('canvas');
        t.width = img.width;
        t.height = img.height;
        const x = t.getContext('2d');
        x.drawImage(img, 0, 0);
        x.globalCompositeOperation = 'source-in';
        x.fillStyle = MAGENTA;
        x.fillRect(0, 0, t.width, t.height);
        this.tinted.set(item, t);
        return t;
    }

    drawGuides(m, tinted) {
        const g = this.ctx;
        for (const gd of m.guides) {
            const r = sectorRect(gd.sector, m.sector_size);
            if (!this.visible(r)) continue;
            const img = this.images.get(gd.item);
            if (!img || img === 'loading') continue;
            g.drawImage(tinted ? this.tint(gd.item, img) : img, r.x, r.y, r.w, r.h);
        }
    }

    draw() {
        const g = this.ctx;
        const W = this.canvas.width;
        const H = this.canvas.height;
        this.colors ??= { bg: css('--bg'), panel2: css('--panel-2'), line: css('--line'), line2: css('--line-2'), blue: css('--blue'), muted: css('--muted'), danger: css('--danger') };
        const c = this.colors;
        g.setTransform(1, 0, 0, 1, 0, 0);
        g.globalAlpha = 1;
        g.fillStyle = c.bg;
        g.fillRect(0, 0, W, H);
        if (!this.doc) return;
        const m = this.doc.manifest;
        const dpr = window.devicePixelRatio || 1;
        const umin = Math.min(...m.sectors.map((s) => s.u));
        const vmin = Math.min(...m.sectors.map((s) => s.v));

        g.setTransform(this.k, 0, 0, this.k, this.ox, this.oy);
        for (const s of m.sectors) {
            const r = sectorRect(s, m.sector_size);
            g.fillStyle = c.panel2;
            g.fillRect(r.x, r.y, r.w, r.h);
        }
        g.setTransform(1, 0, 0, 1, 0, 0);
        this.drawGrid(m, c, dpr);

        g.setTransform(this.k, 0, 0, this.k, this.ox, this.oy);
        g.imageSmoothingQuality = 'high';
        if (m.guide_show) {
            g.globalAlpha = (this.opacity ?? m.guide_opacity) / 100;
            this.drawGuides(m, false);
            g.globalAlpha = 1;
        }
        for (const t of m.tiles) {
            const f = this.shown(t);
            const img = this.images.get(t.item);
            const [cx, cy] = F.centre(f);
            g.save();
            g.translate(cx, cy);
            g.rotate((F.angle(f) * Math.PI) / 180);
            if (img && img !== 'loading') g.drawImage(img, -f.w / 2, -f.h / 2, f.w, f.h);
            else {
                g.fillStyle = c.line;
                g.fillRect(-f.w / 2, -f.h / 2, f.w, f.h);
            }
            g.restore();
        }
        if (this.preview) this.drawGuides(m, true);

        // Screen-space strokes and labels.
        g.setTransform(1, 0, 0, 1, 0, 0);
        g.lineWidth = dpr;
        for (const s of m.sectors) {
            const r = sectorRect(s, m.sector_size);
            const [x0, y0] = this.toScreen([r.x, r.y]);
            const [x1, y1] = this.toScreen([r.x + r.w, r.y + r.h]);
            g.strokeStyle = c.line2;
            g.strokeRect(Math.round(x0) + 0.5, Math.round(y0) + 0.5, Math.round(x1 - x0), Math.round(y1 - y0));
            g.fillStyle = c.muted;
            g.font = `${11 * dpr}px ${getComputedStyle(document.body).fontFamily}`;
            g.fillText(String(1001 + (s.u - umin) + 10 * (s.v - vmin)), x0 + 6 * dpr, y0 + 14 * dpr);
        }
        g.strokeStyle = c.blue;
        g.lineWidth = 2 * dpr;
        for (const s of this.selSectors) {
            const r = sectorRect(s, m.sector_size);
            const [x0, y0] = this.toScreen([r.x, r.y]);
            const [x1, y1] = this.toScreen([r.x + r.w, r.y + r.h]);
            g.strokeRect(x0 + dpr, y0 + dpr, x1 - x0 - 2 * dpr, y1 - y0 - 2 * dpr);
        }
        g.lineWidth = dpr;
        for (const t of m.tiles) {
            if (!this.selTiles.has(t.id)) continue;
            const f = this.shown(t);
            const cs = F.corners(f).map((p) => this.toScreen(p));
            g.strokeStyle = c.blue;
            g.beginPath();
            cs.forEach((p, i) => (i ? g.lineTo(p[0], p[1]) : g.moveTo(p[0], p[1])));
            g.closePath();
            g.stroke();
            if (this.selTiles.size === 1) {
                const hs = HANDLE * dpr;
                for (const [sx, sy] of cs) {
                    g.fillStyle = c.bg;
                    g.fillRect(sx - hs, sy - hs, 2 * hs, 2 * hs);
                    g.strokeRect(sx - hs, sy - hs, 2 * hs, 2 * hs);
                }
                this.drawPivot(this.toScreen(F.pivotCanvas(f)), c, dpr);
            }
        }
        this.drawSnapLines(c, dpr);
        this.drawMarquee(dpr);
        this.placePlus();
    }

    /** Snap lines of a move drag (design D4): along the target, spanning it and the moved box. */
    /** The selection frame (design D5): 1 CSS px marching ants, black over white, no fill. */
    drawMarquee(dpr) {
        const d = this.drag;
        if (!d || d.kind !== 'marquee' || !d.started) return;
        const g = this.ctx;
        const a = this.toScreen(d.p0);
        const x0 = Math.round(Math.min(a[0], d.last[0])) + 0.5;
        const y0 = Math.round(Math.min(a[1], d.last[1])) + 0.5;
        const x1 = Math.round(Math.max(a[0], d.last[0])) + 0.5;
        const y1 = Math.round(Math.max(a[1], d.last[1])) + 0.5;
        g.beginPath();
        g.rect(x0, y0, x1 - x0, y1 - y0);
        strokeAnts(g, dpr);
        // Keep the ants marching while the frame is up, even when the pointer rests.
        this.request();
    }

    drawSnapLines(c, dpr) {
        const d = this.drag;
        if (!d || d.kind !== 'move' || !d.lines.length) return;
        const g = this.ctx;
        const b = { x0: d.box.x0 + d.dx, y0: d.box.y0 + d.dy, x1: d.box.x1 + d.dx, y1: d.box.y1 + d.dy };
        g.strokeStyle = c.danger;
        g.lineWidth = dpr;
        g.beginPath();
        for (const l of d.lines) {
            if (l.axis === 'x') {
                const [x, y0] = this.toScreen([l.value, Math.min(l.span[0], b.y0)]);
                const y1 = this.toScreen([0, Math.max(l.span[1], b.y1)])[1];
                const sx = Math.round(x) + 0.5;
                g.moveTo(sx, y0);
                g.lineTo(sx, y1);
            } else {
                const [x0, y] = this.toScreen([Math.min(l.span[0], b.x0), l.value]);
                const x1 = this.toScreen([Math.max(l.span[1], b.x1), 0])[0];
                const sy = Math.round(y) + 0.5;
                g.moveTo(x0, sy);
                g.lineTo(x1, sy);
            }
        }
        g.stroke();
    }

    /** Pivot marker: Lucide `crosshair` geometry, blue over a dark halo. */
    drawPivot(p, c, dpr) {
        const g = this.ctx;
        const k = (PIVOT_SIZE * dpr) / 24;
        g.save();
        g.translate(p[0] - 12 * k, p[1] - 12 * k);
        g.scale(k, k);
        const path = new Path2D();
        const [cx, cy, r] = CROSSHAIR.circle;
        path.arc(cx, cy, r, 0, Math.PI * 2);
        for (const [x1, y1, x2, y2] of CROSSHAIR.lines) {
            path.moveTo(x1, y1);
            path.lineTo(x2, y2);
        }
        g.lineCap = 'round';
        g.lineJoin = 'round';
        g.strokeStyle = c.bg;
        g.lineWidth = 4;
        g.stroke(path);
        g.strokeStyle = c.blue;
        g.lineWidth = 2;
        g.stroke(path);
        g.restore();
    }

    /** "+" buttons on the free sides of the single selected sector. */
    placePlus() {
        const layer = this.plusLayer;
        const m = this.doc.manifest;
        const s = this.selSectors.length === 1 ? this.selSectors[0] : null;
        const wanted = s ? SIDES.filter((d) => !m.sectors.some((c) => c.u === s.u + d.du && c.v === s.v + d.dv)) : [];
        const dpr = window.devicePixelRatio || 1;
        layer.replaceChildren(
            ...wanted.map((d) => {
                const r = sectorRect(s, m.sector_size);
                const cx = r.x + r.w / 2 + (d.du * r.w) / 2;
                const cy = r.y + r.h / 2 - (d.dv * r.h) / 2;
                const [sx, sy] = this.toScreen([cx, cy]);
                const off = 16;
                const b = document.createElement('button');
                b.type = 'button';
                b.className = 'icon-btn';
                b.title = `Add a sector ${d.side === 'up' ? 'above' : d.side === 'down' ? 'below' : `to the ${d.side}`}`;
                b.setAttribute('aria-label', b.title);
                b.innerHTML = PLUS_ICON;
                b.style.left = `${sx / dpr + d.du * off}px`;
                b.style.top = `${sy / dpr - d.dv * off}px`;
                b.addEventListener('click', () => this.hooks.edit({ op: 'add_sector', sector: s, side: d.side }));
                return b;
            }),
        );
    }
}
