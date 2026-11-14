// ATLAS canvas (design D6 of add-texture-atlas): sectors, tiles and the "+" buttons on one
// 2D canvas with pan and zoom. It never changes the workspace itself: every committed edit
// goes through `hooks.edit`, and the canvas redraws from the document Rust returns.
// Canvas space matches Rust: x right, y down; sector (u, v) spans y in [-(v+1)S, -vS).

import { PLUS_ICON } from '../icons.js';

const HANDLE = 6; // half size of a corner handle, screen px
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

export class AtlasCanvas {
    /**
     * `hooks.edit(edit)` commits an edit; `hooks.select()` reports selection changes;
     * `hooks.status(text)` shows live feedback (tile size while scaling).
     */
    constructor(stage, canvas, plusLayer, hooks) {
        this.stage = stage;
        this.canvas = canvas;
        this.ctx = canvas.getContext('2d');
        this.plusLayer = plusLayer;
        this.hooks = hooks;
        this.doc = null;
        this.images = new Map(); // item -> ImageBitmap | 'loading'
        this.selTiles = new Set();
        this.selSectors = [];
        this.k = 0.25;
        this.ox = 0;
        this.oy = 0;
        this.fitted = false;
        this.drag = null; // { kind: 'move'|'scale'|'pan', ... }
        this.space = false;
        this.pending = false;
        this.colors = null;
        new ResizeObserver(() => this.resize()).observe(stage);
        canvas.addEventListener('pointerdown', (e) => this.onDown(e));
        window.addEventListener('pointermove', (e) => this.onMove(e));
        window.addEventListener('pointerup', (e) => this.onUp(e));
        canvas.addEventListener('wheel', (e) => this.onWheel(e), { passive: false });
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

    loadImages() {
        for (const t of this.doc.manifest.tiles) {
            if (this.images.has(t.item)) continue;
            this.images.set(t.item, 'loading');
            fetch(`http://ripper.localhost/atlas/${this.doc.id}/${t.item}?max=2048`)
                .then((r) => (r.ok ? r.blob() : Promise.reject(new Error(r.status))))
                .then((b) => createImageBitmap(b))
                .then((bmp) => {
                    this.images.set(t.item, bmp);
                    this.request();
                })
                .catch(() => this.images.delete(t.item));
        }
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
        const rs = m.sectors.map((s) => sectorRect(s, m.sector_size)).concat(m.tiles.map((t) => ({ x: t.x, y: t.y, w: t.w, h: t.h })));
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

    /** Displayed rect of a tile, including an in-progress move or scale. */
    tileRect(t) {
        const d = this.drag;
        if (d && d.kind === 'move' && this.selTiles.has(t.id)) {
            return { x: t.x + d.dx, y: t.y + d.dy, w: t.w, h: t.h };
        }
        if (d && d.kind === 'scale' && d.id === t.id && d.rect) return d.rect;
        return { x: t.x, y: t.y, w: t.w, h: t.h };
    }

    tileAt(p) {
        const tiles = this.doc.manifest.tiles;
        for (let i = tiles.length - 1; i >= 0; i--) {
            if (inRect(p, this.tileRect(tiles[i]))) return tiles[i];
        }
        return null;
    }

    sectorAt(p) {
        const m = this.doc.manifest;
        return m.sectors.find((s) => inRect(p, sectorRect(s, m.sector_size))) ?? null;
    }

    corners(r) {
        return {
            top_left: [r.x, r.y],
            top_right: [r.x + r.w, r.y],
            bottom_right: [r.x + r.w, r.y + r.h],
            bottom_left: [r.x, r.y + r.h],
        };
    }

    /** Handle under the screen point for the single selected tile: its corner name. */
    handleAt(s) {
        if (this.selTiles.size !== 1) return null;
        const t = this.doc.manifest.tiles.find((x) => this.selTiles.has(x.id));
        if (!t) return null;
        const r = HANDLE * (window.devicePixelRatio || 1) + 2;
        for (const [name, c] of Object.entries(this.corners(this.tileRect(t)))) {
            const p = this.toScreen(c);
            if (Math.abs(p[0] - s[0]) <= r && Math.abs(p[1] - s[1]) <= r) return { tile: t, corner: name };
        }
        return null;
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
            const opposite = { top_left: 'bottom_right', top_right: 'bottom_left', bottom_right: 'top_left', bottom_left: 'top_right' };
            const anchor = opposite[h.corner];
            this.drag = { kind: 'scale', id: h.tile.id, tile: h.tile, anchor, fixed: this.corners(h.tile)[anchor], rect: null };
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
            if (this.selTiles.has(t.id)) this.drag = { kind: 'move', start: p, dx: 0, dy: 0 };
            this.request();
            return;
        }
        const sec = this.sectorAt(p);
        if (sec) {
            if (e.shiftKey) {
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

    onMove(e) {
        if (!this.doc) return;
        const s = this.local(e);
        const d = this.drag;
        if (!d) {
            if (e.target !== this.canvas) return;
            const h = this.handleAt(s);
            const nwse = h && (h.corner === 'top_left' || h.corner === 'bottom_right');
            this.stage.classList.toggle('over-handle-nwse', Boolean(h && nwse));
            this.stage.classList.toggle('over-handle-nesw', Boolean(h && !nwse));
            const t = !h && this.tileAt(this.toCanvas(...s));
            this.stage.classList.toggle('over-tile', Boolean(t));
            return;
        }
        if (d.kind === 'pan') {
            this.ox += s[0] - d.last[0];
            this.oy += s[1] - d.last[1];
            d.last = s;
            this.request();
        } else if (d.kind === 'move') {
            const p = this.toCanvas(...s);
            d.dx = Math.round(p[0] - d.start[0]);
            d.dy = Math.round(p[1] - d.start[1]);
            this.request();
        } else if (d.kind === 'scale') {
            const p = this.toCanvas(...s);
            const t = d.tile;
            const ratio = t.h / t.w;
            const [fx, fy] = d.fixed;
            const w = Math.max(1, Math.round(Math.max(Math.abs(p[0] - fx), Math.abs(p[1] - fy) / ratio)));
            const h = Math.max(1, Math.round(w * ratio));
            const x = d.anchor.endsWith('left') ? fx : fx - w;
            const y = d.anchor.startsWith('top') ? fy : fy - h;
            d.rect = { x, y, w, h };
            this.hooks.status(`${w} × ${h}`);
            this.request();
        }
    }

    onUp() {
        const d = this.drag;
        if (!d) return;
        this.drag = null;
        this.stage.classList.remove('is-panning');
        if (d.kind === 'move' && (d.dx !== 0 || d.dy !== 0)) {
            const moves = this.doc.manifest.tiles.filter((t) => this.selTiles.has(t.id)).map((t) => [t.id, t.x + d.dx, t.y + d.dy]);
            // Keep the moved position on screen until the new document arrives.
            for (const t of this.doc.manifest.tiles) {
                if (this.selTiles.has(t.id)) {
                    t.x += d.dx;
                    t.y += d.dy;
                }
            }
            this.hooks.edit({ op: 'move', moves });
        } else if (d.kind === 'scale' && d.rect) {
            const t = this.doc.manifest.tiles.find((x) => x.id === d.id);
            if (t) Object.assign(t, d.rect);
            this.hooks.edit({ op: 'scale', id: d.id, w: d.rect.w, anchor: d.anchor });
        }
        this.request();
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

    draw() {
        const g = this.ctx;
        const W = this.canvas.width;
        const H = this.canvas.height;
        this.colors ??= { bg: css('--bg'), panel2: css('--panel-2'), line: css('--line'), line2: css('--line-2'), blue: css('--blue'), muted: css('--muted') };
        const c = this.colors;
        g.setTransform(1, 0, 0, 1, 0, 0);
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
        g.imageSmoothingQuality = 'high';
        for (const t of m.tiles) {
            const r = this.tileRect(t);
            const img = this.images.get(t.item);
            g.save();
            g.translate(r.x + r.w / 2, r.y + r.h / 2);
            g.rotate((t.rot * Math.PI) / 2);
            const odd = t.rot % 2 === 1;
            const [dw, dh] = odd ? [r.h, r.w] : [r.w, r.h];
            if (img && img !== 'loading') g.drawImage(img, -dw / 2, -dh / 2, dw, dh);
            else {
                g.fillStyle = c.line;
                g.fillRect(-dw / 2, -dh / 2, dw, dh);
            }
            g.restore();
        }

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
            const r = this.tileRect(t);
            const [x0, y0] = this.toScreen([r.x, r.y]);
            const [x1, y1] = this.toScreen([r.x + r.w, r.y + r.h]);
            g.strokeStyle = c.blue;
            g.strokeRect(x0, y0, x1 - x0, y1 - y0);
            if (this.selTiles.size === 1) {
                const hs = HANDLE * dpr;
                for (const p of Object.values(this.corners(r))) {
                    const [sx, sy] = this.toScreen(p);
                    g.fillStyle = c.bg;
                    g.fillRect(sx - hs, sy - hs, 2 * hs, 2 * hs);
                    g.strokeRect(sx - hs, sy - hs, 2 * hs, 2 * hs);
                }
            }
        }
        this.placePlus();
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
