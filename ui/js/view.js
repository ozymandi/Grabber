// Main canvas image layer (design D7): WebGL2 textured rectangle under a view transform.
// screen (CSS px) = image (px) * scale + [tx, ty]

const VERT = `#version 300 es
in vec2 a_pos;          // image pixels
uniform vec2 u_img;     // image size in pixels
uniform vec2 u_t;       // translate, CSS px
uniform float u_s;      // scale
uniform vec2 u_view;    // viewport, CSS px
out vec2 v_uv;
void main() {
    vec2 css = a_pos * u_s + u_t;
    vec2 clip = css / u_view * 2.0 - 1.0;
    gl_Position = vec4(clip.x, -clip.y, 0.0, 1.0);
    v_uv = a_pos / u_img;
}`;

const FRAG = `#version 300 es
precision highp float;
in vec2 v_uv;
uniform sampler2D u_tex;
out vec4 o;
void main() { o = texture(u_tex, v_uv); }`;

function compile(gl, type, src) {
    const s = gl.createShader(type);
    gl.shaderSource(s, src);
    gl.compileShader(s);
    if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s));
    return s;
}

export function program(gl, vert, frag) {
    const p = gl.createProgram();
    gl.attachShader(p, compile(gl, gl.VERTEX_SHADER, vert));
    gl.attachShader(p, compile(gl, gl.FRAGMENT_SHADER, frag));
    gl.linkProgram(p);
    if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(p));
    return p;
}

/**
 * Uploads straight-alpha RGBA pixels as a mipmapped texture. Sources larger than the GPU
 * limit are box-downscaled for display only; export always reads full resolution in Rust.
 * Returns the texture and the downscale factor used.
 */
export function uploadTexture(gl, src) {
    const max = gl.getParameter(gl.MAX_TEXTURE_SIZE);
    let { w, h, pixels } = src;
    let factor = 1;
    while (Math.max(w, h) / factor > max) factor *= 2;
    if (factor > 1) {
        const nw = Math.floor(w / factor);
        const nh = Math.floor(h / factor);
        const out = new Uint8Array(nw * nh * 4);
        const n = factor * factor;
        for (let y = 0; y < nh; y++) {
            for (let x = 0; x < nw; x++) {
                const acc = [0, 0, 0, 0];
                for (let j = 0; j < factor; j++) {
                    const row = ((y * factor + j) * w + x * factor) * 4;
                    for (let i = 0; i < factor * 4; i++) acc[i & 3] += pixels[row + i];
                }
                const o = (y * nw + x) * 4;
                for (let k = 0; k < 4; k++) out[o + k] = acc[k] / n;
            }
        }
        pixels = out;
        w = nw;
        h = nh;
    }
    const tex = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
    gl.generateMipmap(gl.TEXTURE_2D);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    const aniso = gl.getExtension('EXT_texture_filter_anisotropic');
    if (aniso) {
        const maxA = gl.getParameter(aniso.MAX_TEXTURE_MAX_ANISOTROPY_EXT);
        gl.texParameterf(gl.TEXTURE_2D, aniso.TEXTURE_MAX_ANISOTROPY_EXT, Math.min(8, maxA));
    }
    return { tex, factor };
}

export class View {
    constructor(canvas) {
        this.canvas = canvas;
        const gl = canvas.getContext('webgl2', { alpha: false, antialias: false, premultipliedAlpha: false });
        if (!gl) throw new Error('WebGL2 is not available');
        this.gl = gl;
        this.prog = program(gl, VERT, FRAG);
        this.buf = gl.createBuffer();
        this.vao = gl.createVertexArray();
        gl.bindVertexArray(this.vao);
        gl.bindBuffer(gl.ARRAY_BUFFER, this.buf);
        const loc = gl.getAttribLocation(this.prog, 'a_pos');
        gl.enableVertexAttribArray(loc);
        gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);
        this.u = Object.fromEntries(
            ['u_img', 'u_t', 'u_s', 'u_view', 'u_tex'].map((n) => [n, gl.getUniformLocation(this.prog, n)]),
        );
        this.img = null; // { w, h, tex }
        this.scale = 1;
        this.tx = 0;
        this.ty = 0;
        this.cssW = 1;
        this.cssH = 1;
        this.maxTexture = gl.getParameter(gl.MAX_TEXTURE_SIZE);
    }

    /** Resizes the backing store to the element size times devicePixelRatio. */
    resize() {
        const r = this.canvas.getBoundingClientRect();
        const dpr = window.devicePixelRatio || 1;
        this.cssW = Math.max(1, r.width);
        this.cssH = Math.max(1, r.height);
        this.canvas.width = Math.round(this.cssW * dpr);
        this.canvas.height = Math.round(this.cssH * dpr);
    }

    setImage(src) {
        const gl = this.gl;
        if (this.img) gl.deleteTexture(this.img.tex);
        if (!src) {
            this.img = null;
            return;
        }
        const { tex, factor } = uploadTexture(gl, src);
        this.img = { w: src.w, h: src.h, tex, factor };
        gl.bindBuffer(gl.ARRAY_BUFFER, this.buf);
        const { w, h } = src;
        gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([0, 0, w, 0, 0, h, w, h]), gl.STATIC_DRAW);
    }

    /** Fits the whole image into the view with a small margin. */
    fit() {
        if (!this.img) return;
        const m = 24;
        const s = Math.min((this.cssW - 2 * m) / this.img.w, (this.cssH - 2 * m) / this.img.h);
        this.scale = Math.max(s, 1e-3);
        this.tx = (this.cssW - this.img.w * this.scale) / 2;
        this.ty = (this.cssH - this.img.h * this.scale) / 2;
    }

    /** Zooms by `factor`, keeping the image point under (sx, sy) fixed. */
    zoomAt(sx, sy, factor) {
        const s = Math.min(Math.max(this.scale * factor, 0.02), 64);
        const k = s / this.scale;
        this.tx = sx - (sx - this.tx) * k;
        this.ty = sy - (sy - this.ty) * k;
        this.scale = s;
    }

    panBy(dx, dy) {
        this.tx += dx;
        this.ty += dy;
    }

    toImage(sx, sy) {
        return [(sx - this.tx) / this.scale, (sy - this.ty) / this.scale];
    }

    toScreen(p) {
        return [p[0] * this.scale + this.tx, p[1] * this.scale + this.ty];
    }

    draw() {
        const gl = this.gl;
        gl.viewport(0, 0, this.canvas.width, this.canvas.height);
        gl.clearColor(0x1e / 255, 0x1e / 255, 0x1e / 255, 1);
        gl.clear(gl.COLOR_BUFFER_BIT);
        if (!this.img) return;
        gl.useProgram(this.prog);
        gl.bindVertexArray(this.vao);
        gl.activeTexture(gl.TEXTURE0);
        gl.bindTexture(gl.TEXTURE_2D, this.img.tex);
        // Crisp pixels when zoomed in, so corners can be placed on exact pixels.
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, this.scale >= 2 ? gl.NEAREST : gl.LINEAR);
        gl.uniform2f(this.u.u_img, this.img.w, this.img.h);
        gl.uniform2f(this.u.u_t, this.tx, this.ty);
        gl.uniform1f(this.u.u_s, this.scale);
        gl.uniform2f(this.u.u_view, this.cssW, this.cssH);
        gl.uniform1i(this.u.u_tex, 0);
        gl.enable(gl.BLEND);
        gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
        gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    }
}
