// Development only: when the page is opened outside the app with ?demo, generate a test
// image so canvas interaction can be exercised in a plain browser. Never used in the app.

/** 1600 x 1000 grey image with a blue/white checkerboard drawn in perspective. */
export function demoSource() {
    const w = 1600;
    const h = 1000;
    const c = document.createElement('canvas');
    c.width = w;
    c.height = h;
    const g = c.getContext('2d');
    g.fillStyle = '#6e6a64';
    g.fillRect(0, 0, w, h);
    // Board corners in the image (TL, TR, BR, BL).
    const quad = [[420, 180], [1180, 260], [1260, 820], [330, 760]];
    const lerp = (a, b, t) => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
    const at = (u, v) => lerp(lerp(quad[0], quad[1], u), lerp(quad[3], quad[2], u), v);
    const n = 8;
    for (let j = 0; j < n; j++) {
        for (let i = 0; i < n; i++) {
            const p = [at(i / n, j / n), at((i + 1) / n, j / n), at((i + 1) / n, (j + 1) / n), at(i / n, (j + 1) / n)];
            g.beginPath();
            p.forEach(([x, y], k) => (k ? g.lineTo(x, y) : g.moveTo(x, y)));
            g.closePath();
            g.fillStyle = (i + j) % 2 ? '#18a0fb' : '#ffffff';
            g.fill();
        }
    }
    const pixels = new Uint8Array(g.getImageData(0, 0, w, h).data.buffer);
    return { w, h, stem: 'demo', rev: 1, pixels, ms: 0, quad };
}
