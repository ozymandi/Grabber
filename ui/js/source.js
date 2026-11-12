// Source pixels over the ripper:// protocol (design D4): raw RGBA8, no decoding in the WebView.

/** Tauri's invoke, or a stub that rejects when the page is opened outside the app. */
export function invoke(cmd, args) {
    const core = window.__TAURI__?.core;
    if (!core) return Promise.reject(new Error('not running inside ripper'));
    return core.invoke(cmd, args);
}

export const inApp = () => Boolean(window.__TAURI__?.core);

// On Windows, WebView2 serves custom protocols as http://<scheme>.localhost/.
const SOURCE_URL = 'http://ripper.localhost/source.rgba';

/** Fetches the current source pixels. `info` is the SourceInfo returned by Rust. */
export async function fetchSource(info) {
    const t0 = performance.now();
    const res = await fetch(`${SOURCE_URL}?rev=${info.rev}`);
    if (!res.ok) throw new Error(`source request failed: ${res.status}`);
    const buf = await res.arrayBuffer();
    const w = Number(res.headers.get('X-Width'));
    const h = Number(res.headers.get('X-Height'));
    if (w !== info.w || h !== info.h || buf.byteLength !== w * h * 4) {
        throw new Error(`source size mismatch: ${w}x${h}, ${buf.byteLength} bytes`);
    }
    return { w, h, stem: info.stem, rev: info.rev, pixels: new Uint8Array(buf), ms: performance.now() - t0 };
}

export const getLaunch = () => invoke('get_launch');
export const loadPath = (path) => invoke('load_path', { path });
export const pasteClipboard = () => invoke('paste_clipboard');
export const log = (message) => invoke('ui_log', { message }).catch(() => console.log(message));
