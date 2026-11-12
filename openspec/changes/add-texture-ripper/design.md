# Design

## Context

`E:\Projects\Rust` holds only OpenSpec scaffolding: no code, no git. The machine has:
- Visual Studio 2026 with the MSVC C++ tools;
- the WebView2 runtime 157;
- Node 24;
- ShareX 22.4.0;
- Google Drive for desktop, mounted as `G:\`.

Rust is not installed yet.

Requirements live in `specs/quad-editing`, `specs/quad-unwarp` and `specs/texture-export`. Motivation is in proposal.md.

ShareX behaviour, checked in its source (`ExternalProgram.Run`, `WorkerTask`):
- It runs the action and blocks in `WaitForExit`.
- If the derived `$output` file exists afterwards, the capture continues with that file. Otherwise it continues with the original.

The UI must copy the O'Tool design system one to one. Sources:
- `E:\Projects\Illustrator\Plugins\O'Frame\css\main.css`: tokens and components.
- `E:\Projects\O'Tool\O'Zometrix\DESIGN_SYSTEM.md`: rules.
- `E:\Projects\O'Tool\O'Zometrix\ui.html`: help overlay and status icon actions, which O'Frame lacks.

## Goals / Non-Goals

**Goals:**
- Go from a ShareX capture to a flattened texture in a few seconds: start, four clicks, Enter.
- Fluid editing on 4K sources: no lag while dragging corners.
- Export quality better than a naive bilinear warp when the quad is shrunk.
- UI indistinguishable from an O'Tool panel: same tokens, metrics and controls.

**Non-Goals:**
- Undo/redo history. Quads are cheap to redo, and this can be added later without changing the export contract.
- Multiple source images in one session.
- Installer, auto-update, code signing. A portable exe is enough for v1.
- macOS and Linux.

## Decisions

### D1. Tauri 2 with a plain static frontend
Rust core plus a WebView2 UI written in plain HTML, CSS and ES modules. No bundler and no npm. The Tauri CLI is installed with `cargo install tauri-cli`.

- **Why:** the O'Tool system is CSS (tokens, gradients, 24/26/32 px controls, Segoe UI 11 px). In WebView2 it renders exactly as in the CEP panels, which are also Chromium.
- **Alternative: egui.** Rejected. Every component would have to be redrawn, and its text rasteriser differs from Chromium's (no ClearType, different metrics). That breaks the copy-the-design rule.
- **Alternative: a bundler with a framework.** Rejected. It adds a toolchain with no gain for one screen.

### D2. Layout of the repository
```
E:\Projects\Rust\
  src-tauri\            Rust crate (binary "ripper")
    src\main.rs         args, window, command registration
    src\geom.rs         homography, validity, output size rules
    src\warp.rs         export sampler
    src\source.rs       decode file / clipboard, hold RGBA in state
    src\export.rs       PNG encode, clipboard, folders, naming
    src\settings.rs     config.toml load/save
    tests\              geometry and warp tests, fixtures
  ui\
    index.html
    css\otool.css       O'Frame main.css, copied verbatim
    css\app.css         ripper layout only (canvas + right column)
    js\main.js          state, panels, shortcuts
    js\view.js          main canvas: WebGL image + 2D overlay for quads
    js\preview.js       WebGL preview of the active quad
    js\geom.js          homography port of geom.rs (preview only)
    icons\*.svg         vector icons from the O'Tool sources
```

### D3. Who owns what
The UI owns editing state: quads, active index, aspect mode and power-of-two state per quad, view transform. Rust owns:
- the decoded source pixels;
- export;
- settings;
- the launch mode.

Exports are a single command:

`export { kind: "send" | "all", active, quads[] }` → `{ written[], failures[], exitAfter }`

Each quad carries its corners in source pixels, its aspect mode and its power-of-two flag. **Rust recomputes the output size itself** from those inputs, so the file never depends on numbers computed in JS.

### D4. Getting pixels into the WebView without JSON
A source can be up to ~33 MB of RGBA at 4K. It is never sent through JSON IPC:
- A custom URI scheme is registered: `ripper://`, which WebView2 on Windows serves as `http://ripper.localhost/`.
- `GET /source.rgba` returns the raw RGBA bytes. The width and height go in headers.
- The UI calls `fetch` to get an `ArrayBuffer` and uploads it with `texImage2D` into each WebGL context (main view, preview). The browser never decodes anything.
- Rust decodes the file or clipboard image once with the `image` crate and keeps it in `State<Mutex<Source>>`.
- **Alternative:** base64 or a JSON array of bytes. Rejected: seconds of latency at 4K.
- **Alternative:** the `asset:` protocol pointing at the file. Rejected: it does not cover clipboard sources, and it would decode twice.

### D5. Homography: closed form, one source of truth with a parity test
The square→quad mapping uses Heckbert's closed form. It is inverted with the adjugate in f64. There is no general 8×8 solve, so there is no pivoting and no ill-conditioning surprises.

`geom.rs` is the authority. `geom.js` is a line-for-line port used only for the preview.

A shared fixture, `src-tauri/tests/fixtures/quads.json`, lists quads with their expected matrices and sizes. Two tests check against it:
- the Rust tests;
- a small Node script, `ui/js/geom.test.mjs`, run with `node`.

Both must agree within 1e-9. This enforces "preview matches export geometry".

Validity check (spec: degenerate quads):
- the quad must be convex;
- every cross product of consecutive edges must have the same sign;
- the area must be at least 4 px²;
- no interior angle may be under 2°.

### D6. Export sampler: adaptive supersampling
For each output pixel:
1. Map the pixel centre through the inverse homography.
2. Estimate the local footprint from the Jacobian of the mapping, separately along the output x and y axes: nu = clamp(ceil(footprint along x), 1, 8), nv likewise along y.
3. Average an nu×nv grid of bilinear samples. Per-axis counts give the same quality as a square grid sized by the longer side, with far fewer samples on steep quads.
4. Treat out-of-bounds samples as transparent black.
5. Do the sums in premultiplied alpha, then un-premultiply.

Rows run in parallel with `rayon`. Measured: 12.3 ms for a 2048² output from a 3840×2160 source (release, Ryzen 9 9950X), against a 150 ms target. Beyond 8× shrink along an axis the sample count is capped and quality degrades gradually; that is outside normal use, since the default size follows the quad's own edges.

**Alternative: a mip pyramid with trilinear filtering.** Rejected for export. It blurs anisotropic footprints, and a strongly tilted quad shrinks far more along one axis than the other, so the result would be soft. A pyramid is fine for the preview (D7).

### D7. Preview in WebGL2
The preview canvas draws a full quad. The fragment shader maps the output uv to the source through the homography from `geom.js`.

- The texture is mipmapped (WebGL2 allows non-power-of-two mipmaps), with `LINEAR_MIPMAP_LINEAR` filtering and anisotropic filtering when `EXT_texture_filter_anisotropic` is present.
- The preview canvas keeps the quad's output aspect and fits the panel.
- It redraws on every corner move via `requestAnimationFrame`.

The main view is also WebGL: one textured rectangle with the view transform. Quads, handles and the in-progress polyline go on a 2D overlay canvas, so handles keep a constant screen size and sharp 1 px O'Tool lines.

If the source exceeds `MAX_TEXTURE_SIZE`, the UI uploads a downscaled copy for display only. Export always reads full resolution in Rust.

### D8. Launch modes and exit
- `argv[1]` is the source; `argv[2]` switches on ShareX mode, with that path as the output.
- In ShareX mode a successful send writes the output file. The app exits with code 0 only when all writes succeeded. Any folder failure keeps the window open with a warning (spec: unavailable folder).
- The window is created hidden, with background `#1e1e1e`. It is shown on the UI's `ready` event, so no white flash appears.
- Each ShareX capture starts its own process; there is no single-instance lock. ShareX waits for each one anyway.
- The release build uses the `windows` subsystem, so no console window appears.

### D9. Clipboard
`arboard` puts the RGBA image on the clipboard. On Windows (arboard 3.7) `set_image` writes both a `CF_DIBV5` bitmap, which many apps read without alpha, and the encoded PNG under the registered clipboard format named `PNG`, the convention Photoshop, Figma and browsers use, so transparency survives. No separate `clipboard-win` dependency is needed.

Data is written immediately, not as delayed rendering, so it stays on the clipboard after the process exits in ShareX mode.

### D10. Settings
`%APPDATA%\ripper\config.toml` stores:
- `local_dir`, `local_on`;
- `drive_dir`, `drive_on`;
- `aspect_mode`, `aspect_custom`;
- `pot`;
- the last window size.

It is written on every change, atomically: to a temp file, then renamed. A missing or corrupt file falls back to defaults: both folders off with empty paths, EDGES, power of two off. A corrupt file is renamed to `config.bad.toml`. The folder picker uses `tauri-plugin-dialog`.

### D11. UI layout and interaction, O'Tool rules applied
- **Layout:** the canvas fills the left; a right column 320 px wide, the O'Frame panel's default width, holds the `.panel` cards. The column scrolls with O'Tool's scrollbar style. The `.app` shell uses 8 px padding and 8 px gaps.
- **Panels:**
  - Source: OPEN and PASTE, a `btn-row`.
  - Quads: list rows with W × H and an SVG remove icon.
  - Aspect: `seg-row` with EDGES | 1:1 | 4:3 | 2:1 | CUSTOM. The CUSTOM fields go in a `field-grid-2` and appear only for CUSTOM. A Power of two checkbox follows.
  - Preview.
  - Output: per slot, a checkbox, a path field and a browse icon button. SAVE ALL and COPY form a `btn-row`, and SEND ACTIVE is the `button.primary` action.
  - At the bottom, always visible, the status block: `status-indicator`, status text mirrored into `title`, and a help icon.
- **Help:** explanations and the shortcut list live in the help overlay, built from O'Zometrix `ui.html` with the "part of O'Tool" badge. No inline paragraphs.
- **Canvas colours:** taken from the tokens. Every quad line is a 1 px coloured line over a 3 px dark halo, so it stays visible on any image.
  - Quad edges: `--muted`. `--line-2` was planned but disappears on grey photos.
  - Active quad: `--blue`, with handles filled with `#dbeeff` like the slider thumb.
  - Invalid quad: `--danger`.
  - The first corner (the texture's top-left) has a ring around its handle.
  - Canvas background: `--bg`.
- **Status colours:** O'Tool has no warning token, so warnings (for example a folder that could not be written) use `--danger` like errors.
- **Icons:** SVG only, reusing O'Tool glyphs where they exist (reset ×, chevron). Missing ones (browse folder, help, remove) are drawn at the same 12 px box, stroke and colour.
- **Shortcuts:**
  - Enter — send;
  - Ctrl+S — save all;
  - Ctrl+V — paste;
  - Esc — cancel the unfinished quad, otherwise exit;
  - Delete — remove the active quad;
  - Space+drag or middle-drag — pan;
  - wheel — zoom;
  - F — fit;
  - Tab — next quad.
- **Quad corner order** (spec: corner order): corners are taken in click order. If the user clicks counter-clockwise, they are reordered clockwise while keeping the first click as top-left. Without this, a reversed click order would mirror the texture.

## Risks / Trade-offs

- **Startup slower than a native GUI** (~0.3–0.5 s) → the window is hidden until ready, and the source is decoded in parallel with WebView start-up.
- **Preview and export geometry drift** → the shared fixture parity test (D5). The size is computed only in Rust for the file.
- **The O'Tool CSS diverges if O'Frame changes later** → `otool.css` is a verbatim copy with its source path and date in a header comment. App-specific overrides live only in `app.css`, never inside the copy.
- **Pixel match with O'Tool cannot be proven by eye** → a side-by-side screenshot comparison of shared components (seg-row, buttons, inputs, checkbox, status row) against O'Frame. Any deviation is reported to the designer, not silently accepted.
- **Clipboard alpha lost in some apps** → the PNG clipboard format (D9). Apps that read only the DIB will still show the image without alpha. That is a limitation of those apps.
- **A huge source (8K+) exceeds the GPU texture limit** → a downscaled display copy (D7); export stays full resolution.
- **ShareX blocks its task queue while the window is open** → this is how ShareX actions work (`WaitForExit`). It is documented in help: finish or Esc before the next capture is processed.
- **Edge-based size underestimates the far side of steep quads** → the aspect lock covers it. Automatic estimation was rejected because ShareX region crops break the principal-point assumption.

## Migration Plan

This is a new tool, so nothing needs migrating. Rollout:
1. `cargo tauri build` produces a portable `ripper.exe`.
2. Copy it to a fixed path.
3. Add the ShareX action on a dedicated hotkey, so ordinary captures are unaffected:
   - arguments `"$input" "$output"`;
   - Output extension `tex.png`. ShareX builds `$output` as the input's folder and name plus this extension; with plain `png` the output path equals the input path and the original capture would be overwritten;
   - After capture: Save image to file and Perform actions on, and no upload.

Set up on this machine (2026-11-13): hotkey Ctrl+Shift+F11, region capture, task description `ripper`, pointing at `src-tauri\target\release\ripper.exe`. Backups of the previous ShareX config sit next to it as `*.json.before-ripper`.

To roll back, delete the `ripper` hotkey in ShareX or restore the backups while ShareX is closed.

## Open Questions

- **Product name and icon** within the O'Tool family. It only affects the window title, the exe name and the config folder name, and those are easy to rename before release.
- **Exact default window size** on first run, before one is remembered. 1440×900 is assumed.
