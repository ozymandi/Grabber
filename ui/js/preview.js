// Live preview of the active quad (design D7): a WebGL2 fragment shader maps every output
// pixel through the same homography Rust uses for export (geom.js, checked against geom.rs).

import * as geom from './geom.js';
import { outline } from './shape.js';
import { activeQuad, quadInfo, state } from './state.js';
import { program, uploadTexture } from './view.js';

const VERT = `#version 300 es
in vec2 a_pos;      // 0..1 over the output
out vec2 v_out;
uniform vec2 u_size; // output size in pixels
void main() {
    v_out = a_pos * u_size;
    gl_Position = vec4(a_pos.x * 2.0 - 1.0, 1.0 - a_pos.y * 2.0, 0.0, 1.0);
}`;

// Projective lookup; anything mapping outside the source is transparent, as in export.
const FRAG = `#version 300 es
precision highp float;
in vec2 v_out;
uniform mat3 u_h;    // output px -> source px
uniform vec2 u_img;  // source size
uniform sampler2D u_tex;
uniform sampler2D u_mask; // shape coverage over the output (add-quad-shapes D4)
uniform vec2 u_size;
uniform bool u_masked;
out vec4 o;
void main() {
    vec3 p = u_h * vec3(v_out, 1.0);
    vec2 s = p.xy / p.z;
    if (s.x < 0.0 || s.y < 0.0 || s.x > u_img.x || s.y > u_img.y) { o = vec4(0.0); return; }
    vec4 c = texture(u_tex, s / u_img);
    float a = c.a * (u_masked ? texture(u_mask, v_out / u_size).a : 1.0);
    o = vec4(c.rgb * a, a);
}`;

export class Preview {
    constructor() {
        this.box = document.getElementById('previewBox');
        this.canvas = document.getElementById('preview');
        this.empty = document.getElementById('previewEmpty');
        this.sizeLabel = document.getElementById('previewSize');
        const gl = this.canvas.getContext('webgl2', { alpha: true, premultipliedAlpha: true, antialias: false });
        if (!gl) throw new Error('WebGL2 is not available');
        this.gl = gl;
        this.prog = program(gl, VERT, FRAG);
        this.vao = gl.createVertexArray();
        gl.bindVertexArray(this.vao);
        const buf = gl.createBuffer();
        gl.bindBuffer(gl.ARRAY_BUFFER, buf);
        gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([0, 0, 1, 0, 0, 1, 1, 1]), gl.STATIC_DRAW);
        const loc = gl.getAttribLocation(this.prog, 'a_pos');
        gl.enableVertexAttribArray(loc);
        gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);
        this.u = Object.fromEntries(['u_h', 'u_img', 'u_size', 'u_tex', 'u_mask', 'u_masked'].map((n) => [n, gl.getUniformLocation(this.prog, n)]));
        this.tex = null;
        // Shape mask: drawn by Canvas2D at the preview's pixel size and uploaded only when
        // the shape or that size changes, so corner drags cost what they did (design D4).
        this.maskCanvas = document.createElement('canvas');
        this.maskTex = gl.createTexture();
        gl.bindTexture(gl.TEXTURE_2D, this.maskTex);
        for (const [k, v] of [[gl.TEXTURE_MIN_FILTER, gl.LINEAR], [gl.TEXTURE_MAG_FILTER, gl.LINEAR], [gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE], [gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE]]) {
            gl.texParameteri(gl.TEXTURE_2D, k, v);
        }
        this.maskKey = '';
        this.frame = 0;
        new ResizeObserver(() => this.request()).observe(this.box);
    }

    setImage(src) {
        const gl = this.gl;
        if (this.tex) gl.deleteTexture(this.tex);
        this.tex = src ? uploadTexture(gl, src).tex : null;
        if (this.tex) gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
        this.request();
    }

    request() {
        if (this.frame) return;
        this.frame = requestAnimationFrame(() => {
            this.frame = 0;
            this.draw();
        });
    }

    showEmpty(text) {
        this.empty.textContent = text;
        this.empty.hidden = false;
        this.canvas.style.display = 'none';
    }

    draw() {
        const q = activeQuad();
        if (!q || !state.source || !this.tex) {
            this.sizeLabel.textContent = '';
            this.showEmpty('No quad');
            return;
        }
        const info = quadInfo(q);
        this.sizeLabel.textContent = `${info.w} × ${info.h}`;
        const m = info.invalid ? null : geom.outputToSource(info.cw, info.w, info.h);
        if (!m) {
            this.showEmpty('Invalid quad');
            return;
        }
        this.empty.hidden = true;

        // Fit the output aspect into the box (content box, inside its 1px border).
        const bw = this.box.clientWidth;
        const bh = this.box.clientHeight;
        const k = Math.min(bw / info.w, bh / info.h);
        const cw = Math.max(1, info.w * k);
        const ch = Math.max(1, info.h * k);
        const dpr = window.devicePixelRatio || 1;
        Object.assign(this.canvas.style, {
            display: 'block',
            width: `${cw}px`,
            height: `${ch}px`,
            left: `${(bw - cw) / 2}px`,
            top: `${(bh - ch) / 2}px`,
        });
        this.canvas.width = Math.round(cw * dpr);
        this.canvas.height = Math.round(ch * dpr);

        const gl = this.gl;
        gl.viewport(0, 0, this.canvas.width, this.canvas.height);
        gl.clearColor(0, 0, 0, 0);
        gl.clear(gl.COLOR_BUFFER_BIT);
        gl.useProgram(this.prog);
        gl.bindVertexArray(this.vao);
        gl.activeTexture(gl.TEXTURE0);
        gl.bindTexture(gl.TEXTURE_2D, this.tex);
        // mat3 uniforms are column-major: transpose the row-major homography.
        gl.uniformMatrix3fv(this.u.u_h, false, [m[0][0], m[1][0], m[2][0], m[0][1], m[1][1], m[2][1], m[0][2], m[1][2], m[2][2]]);
        gl.uniform2f(this.u.u_img, state.source.w, state.source.h);
        gl.uniform2f(this.u.u_size, info.w, info.h);
        gl.uniform1i(this.u.u_tex, 0);
        const masked = q.shape.kind !== 'rect';
        if (masked) this.updateMask(q.shape, this.canvas.width, this.canvas.height);
        gl.activeTexture(gl.TEXTURE1);
        gl.bindTexture(gl.TEXTURE_2D, this.maskTex);
        gl.uniform1i(this.u.u_mask, 1);
        gl.uniform1i(this.u.u_masked, masked ? 1 : 0);
        gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    }

    /** Fills the shape's outline into the mask texture at `w x h` when it has changed. */
    updateMask(shape, w, h) {
        const key = `${w}x${h}:${JSON.stringify(shape)}`;
        if (key === this.maskKey) return;
        this.maskKey = key;
        const c = this.maskCanvas;
        c.width = w;
        c.height = h;
        const g = c.getContext('2d');
        g.clearRect(0, 0, w, h);
        g.beginPath();
        outline(shape, w, h).forEach(([x, y], i) => (i ? g.lineTo(x, y) : g.moveTo(x, y)));
        g.closePath();
        g.fillStyle = '#fff';
        g.fill('nonzero');
        const gl = this.gl;
        gl.activeTexture(gl.TEXTURE1);
        gl.bindTexture(gl.TEXTURE_2D, this.maskTex);
        gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);
        gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
        gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, c);
        gl.activeTexture(gl.TEXTURE0);
    }
}
