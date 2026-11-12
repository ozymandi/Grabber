# Tasks

## 1. Toolchain and scaffolding

- [x] 1.1 Install rustup with the `stable-x86_64-pc-windows-msvc` toolchain (ask the designer before installing). Verify that `cargo --version` and `rustc --version` print versions in a new shell.
- [x] 1.2 Install the Tauri CLI with `cargo install tauri-cli --locked`. Verify that `cargo tauri --version` prints a 2.x version.
- [x] 1.3 Run `git init` in `E:\Projects\Rust` and add a `.gitignore` covering `target/`, `src-tauri/gen/` and `*.bak`. Verify that `git status` shows only intended files.
- [x] 1.4 Create `team.md` (reference to the global roles) and `task.md` (task, decisions, current status, next step). Verify both exist at the project root.
- [x] 1.5 Scaffold the `src-tauri/` crate (binary `ripper`, Tauri 2, `windows` subsystem in release) and the `ui/` static frontend with `frontendDist: ../ui` (D1, D2). Add the dependencies `tauri`, `tauri-plugin-dialog`, `image`, `rayon`, `arboard`, `clipboard-win`, `serde`, `toml`, `dirs`. Verify that `cargo tauri dev` opens an empty window.

## 2. Geometry core (Rust, with tests)

- [x] 2.1 Implement `geom.rs`: Heckbert square→quad homography, adjugate inverse, point mapping (D5). Verify with unit tests that the corners map exactly and that a forward-then-inverse round trip stays within 1e-9.
- [x] 2.2 Implement quad validity: convex, consistent winding, area ≥ 4 px², angles ≥ 2°. Add clockwise reordering that keeps the first corner. Verify with tests for a bow-tie, a collinear quad, a tiny quad and a counter-clockwise input (spec quad-unwarp: degenerate quads, corner order).
- [x] 2.3 Implement output size rules: EDGES, 1:1, 4:3, 2:1, CUSTOM, then power-of-two snapping with ties rounding up and a minimum of 1. Verify that tests reproduce the spec examples 350×200, 350×350, 600×200 and 256×256.
- [x] 2.4 Write `tests/fixtures/quads.json` with quads, expected matrices and sizes generated from `geom.rs`. Verify with a Rust test that reads it back.

## 3. Export sampler (Rust, with tests)

- [x] 3.1 Implement `warp.rs`: Jacobian-adaptive n×n supersampling (n ≤ 8) of bilinear samples in premultiplied alpha, with transparent output outside the source and `rayon` rows (D6). Verify with tests that warping a synthetic perspective checkerboard yields axis-aligned edges within 1 px (spec scenario "Rectify a known rectangle").
- [x] 3.2 Add quality and edge tests: a strongly minified fine stripe pattern compared against a box-filtered reference (error bound in the test), alpha preserved, and corners outside the image producing transparent pixels. Verify that `cargo test` passes.
- [x] 3.3 Add a timing test (ignored by default) for a 2048² output from a 4K source. Verify that it runs under 150 ms in release on this machine, or record the measured time in `task.md`.

## 4. Source loading and pixel transport

- [x] 4.1 Implement `source.rs`: decode PNG, JPEG, BMP and WebP from a path, and read a clipboard image through `arboard`, into `State<Mutex<Source>>` with the stem name (`clipboard_<yyyyMMdd-HHmmss>` for paste). Verify with unit tests for each format and for a missing file.
- [x] 4.2 Register the `ripper://` protocol serving `/source.rgba` with width and height headers (D4). Expose the commands `load_path` and `paste_clipboard`, returning `{ w, h, stem }` or an error. Verify that in `cargo tauri dev` a 4K PNG loads into an `ArrayBuffer` in under 200 ms (logged in the console).
- [x] 4.3 Parse launch arguments (D8): `argv[1]` is the source, and `argv[2]` switches on ShareX mode with that output path. Verify by running the dev build with one argument and then two, and checking the mode reported to the UI.

## 5. UI shell in the O'Tool design system

- [x] 5.1 Copy O'Frame `css/main.css` verbatim to `ui/css/otool.css` with a header noting its source path and date. Create `app.css` for the layout only: canvas on the left, a 320 px right column, the 8 px shell. Verify that `otool.css` differs from the source only by the header comment.
- [x] 5.2 Build `index.html` with the panels Source, Quads, Aspect, Preview and Output, and the always-visible status block (D11), using O'Tool classes only. Verify visually in the dev build.
- [x] 5.3 Add SVG icons: reuse O'Tool glyphs (reset ×, chevron) and draw browse-folder, help and remove in the same 12 px box, stroke and colour. Verify that no icon is a text glyph or a PNG.
- [x] 5.4 Add the help overlay modelled on O'Zometrix `ui.html`, with the "part of O'Tool" badge, the shortcut list and the ShareX setup steps. Verify that it opens from the help icon and closes with Esc and with an outside click.
- [x] 5.5 Hide the window until the UI is ready, with background `#1e1e1e` (D8). Verify that no white flash appears on 10 cold starts.
- [x] 5.6 Compare a screenshot of the shared components (seg-row, buttons, inputs, checkbox, status row) side by side with O'Frame. Verify that metrics and colours match, and report any deviation to the designer.

## 6. Canvas editing (spec quad-editing)

- [x] 6.1 Implement `view.js`: WebGL image layer with wheel zoom toward the cursor, Space-drag and middle-drag pan, F to fit, and a display-only downscaled copy when the source exceeds `MAX_TEXTURE_SIZE`. Verify that the zoom-toward-cursor and pan scenarios hold on a 4K image.
- [x] 6.2 Implement the 2D overlay: quad creation by four clicks, Esc cancelling an unfinished quad, draggable constant-size handles, colours from the tokens (normal, active, invalid). Verify the scenarios "Create a quad", "Cancel an unfinished quad", "Drag a corner" and "Handles at high zoom".
- [x] 6.3 Implement the Quads panel: rows with W × H, selection, Delete and remove icon, Tab to the next quad, with the active quad always defined. Verify the scenarios "Activate from the list" and "Delete the active quad".
- [x] 6.4 Wire OPEN (dialog), PASTE and Ctrl+V, and drag-and-drop to replace the source and clear the quads. Show an error status for a bad file or an empty clipboard. Verify the scenarios "Paste a screenshot", "Paste with no image" and "Drop a file".

## 7. Live preview (spec quad-unwarp, quad-editing)

- [x] 7.1 Port `geom.rs` to `geom.js` and add `geom.test.mjs`, which checks against `quads.json`. Verify that `node ui/js/geom.test.mjs` passes with tolerance 1e-9.
- [x] 7.2 Implement `preview.js`: WebGL2 projective shader, mipmaps with anisotropic filtering when available, the output aspect kept, an empty state, and an invalid state. Verify that the preview follows a corner drag on a 4K source without visible lag.
- [x] 7.3 Wire the Aspect panel and Power of two for the active quad: the CUSTOM fields show only for CUSTOM, and the sizes update in the Quads panel and the preview. Verify the spec size examples interactively.

## 8. Export, clipboard, folders, settings (spec texture-export)

- [x] 8.1 Implement `settings.rs`: `%APPDATA%\ripper\config.toml`, atomic writes, defaults, and renaming a corrupt file to `config.bad.toml` (D10). Verify with unit tests for the round trip, a missing file and a corrupt file.
- [x] 8.2 Implement the Output panel: two slots, each with a checkbox, a path and a browse button through `tauri-plugin-dialog`, persisting on change. New quads take the last-used aspect mode and power-of-two state. Verify the scenario "Restart keeps folders".
- [x] 8.3 Implement the `export` command (D3). Rust recomputes the sizes, warps and encodes the PNG, applies naming with `_2`, `_3` suffixes, and reports written files and failures per folder. Verify with unit tests for the naming collision and for a folder failure that does not block the others.
- [x] 8.4 Implement the clipboard output: the image through `arboard`, which writes both `CF_DIBV5` and the registered `PNG` format (D9). Verify by pasting into Photoshop or Figma with transparency kept, and checking that the clipboard survives the app exiting.
- [x] 8.5 Wire Enter / SEND ACTIVE, Ctrl+S / SAVE ALL, COPY and Esc / close, with the ShareX-mode exit rules: exit 0 only on full success; stay open on a folder failure; Esc before a send creates no output file. Verify every texture-export scenario manually, including both slots off.

## 9. ShareX integration check

- [x] 9.1 Build the release exe with `cargo tauri build`. Set up a ShareX 22.4 action with arguments `"$input" "$output"`, output extension `png`, and Save image to file plus Perform actions on. Verify that a region capture leads to placing a quad and pressing Enter, after which ShareX continues with the flattened PNG.
- [x] 9.2 In the same setup, press Esc. Verify that ShareX continues with the original capture and that no output file is left.
- [x] 9.3 Turn on the Drive slot with `G:\` unavailable (Drive paused or a bad path). Verify that the clipboard, the output file and Local are written, a warning names Drive, and the window stays open.
- [x] 9.4 Update `task.md` with the status, the measured startup and export times, and the remaining open questions (product name, default window size).

## Workflow follow-up

- Review the result with the designer, then archive the change with `/opsx:archive`.
