# Design

## Context

ripper 1.0.0 is a single-image editor. Its parts:
- **Frontend.** `ui/js/editor.js`, `view.js` and `preview.js` drive two canvases: the WebGL2 image and the overlay. The side panel is static HTML in O'Tool styles.
- **Rust.** It renders and writes exports (`export.rs`, `warp.rs` with Jacobian-adaptive supersampling) and keeps settings in `%APPDATA%\ripper\config.toml`. It serves pixels over the `ripper://` protocol (D4 of the first change).
- **Launches.** A ShareX launch is a separate short-lived process: it starts, exports on Enter and exits.

Build output goes to `C:\cargo-target`. Nothing heavy may live under `E:\Projects`, which Google Drive mirrors.

Requirements are in `specs/atlas-workspace`, `specs/atlas-tiles`, `specs/atlas-export` and the `texture-export` delta. Motivation is in proposal.md.

## Goals / Non-Goals

**Goals:**
- A tile sent from a ShareX capture lands in the current workspace without slowing the capture flow. The atlas write must stay well under a second.
- No lost work. Every edit is on disk at once, and two ripper processes touching one workspace never overwrite each other.
- Exports at least as clean as RIP exports: the same sampler, always from the original.
- The Rust core stays Tauri-free and unit-testable, as in 1.0.0.

**Non-Goals:**
- Real-time collaboration, or more than two processes editing at once beyond the locking below.
- Undo and redo, snapping and guides. Free rotation and UV guides come in 2.1.0.

## Decisions

### D1. Workspace storage: working folder plus `.ripper` archive

Each workspace has a working folder at `%APPDATA%\ripper\workspaces\<id>\`:

```
manifest.json        model (D2), written atomically on every change
items/<tile-id>.png  full-size originals, written once, never modified
manifest.lock        present only while a writer holds the lock (D3)
```

- `%APPDATA%\ripper\workspaces\index.json` maps each working folder to its `.ripper` path, if any, and stores `saved_revision`.
- **Unsaved state.** The workspace has unsaved changes when `manifest.revision != saved_revision`.
- **Current workspace.** Settings gain `current_workspace` (a folder id) and `atlas_on`.
- **The `.ripper` file** is a zip archive. It holds `manifest.json` (deflated) and `items/*.png`, stored without recompression, since PNG is already compressed.
  - SAVE writes `<file>.tmp` and renames it over the target. SAVE AS also updates the index.
  - OPEN looks the path up in the index. If a working folder exists and has unsaved changes, that folder is used: it is newer than the file, and this is how a closed window recovers. Otherwise the archive is unpacked into a fresh folder.
- **Why both, not just the zip.** A ShareX capture would otherwise rewrite a zip full of 4K originals on every Enter, which is too slow and risky if interrupted. Appending one PNG plus a small manifest is fast and safe.

**Dependency.** `zip` becomes a direct dependency. It is already in `Cargo.lock` through `tauri-plugin-updater`.

### D2. Model and coordinates

```
Manifest {
  format: 1,
  revision: u64,             // +1 on every write
  name: String,              // "Untitled" or the file stem
  sector_size: u32,          // 512..8192, power of two
  padding: u32,              // default 8
  sectors: [ {u: i32, v: i32} ],
  active: {u, v} | null,
  tiles: [ { id, item, w0, h0,          // original size
             x, y, w, h,                // canvas rect of the placed tile (after rotation)
             rot: 0|1|2|3,              // quarter turns clockwise
             source: { stem, quad } } ] // provenance for the status and names
}
```

- **Canvas space.** Canvas pixels, x to the right and y down, which matches the screen. Sector `(u, v)` occupies `x in [u*S, (u+1)*S)` and `y in [-(v+1)*S, -v*S)`, so v grows upward as in UDIM. Two helpers convert between the two spaces; nothing else touches the formula.
- **Rect and scale.** A tile's displayed rect is axis-aligned, so `w`/`h` already include the rotation. The scale is `w / (rot odd ? h0 : w0)`.
- **Order.** Later tiles draw on top. Moving a tile does not change the order.

### D3. Two writers: lock, re-read, apply, write

All mutations go through Rust and follow the same sequence:
1. take the lock;
2. read `manifest.json`;
3. apply the operation;
4. write `manifest.json.tmp`;
5. rename it over `manifest.json`;
6. release the lock.

New items are written before the manifest that references them, so a manifest never points at a missing file.

- **Lock.** `manifest.lock` is created with `create_new` and holds the pid and a timestamp.
  - Writers retry for up to 2 s; after that the operation fails, which in ShareX mode is reported per the texture-export delta.
  - A lock older than 10 s is broken: operations hold it for milliseconds, so such a lock belongs to a writer that died. The pid is kept in the file for diagnostics only.
- **UI.** The UI never holds authoritative state. Each command returns the new manifest.
- **Watching for changes.** While ATLAS is visible, the open window checks `manifest.json`'s revision every 500 ms; it is a small file. On change it reloads, which meets the two-second rule.
- **Operations are deltas**, such as "move tile 7 to x, y". A ShareX add and a user move therefore merge instead of clobbering.

- *Alternative: the `notify` crate for file watching.* More moving parts, and Drive or AV tools can delay events. Polling one small file is simpler and predictable.
- *Alternative: a single-instance app with IPC.* That would change the ShareX launch contract of 1.0.0. Rejected.

### D4. Automatic placement (append-only)

A pure function: `place(sector_rect, padding, occupied: [Rect], w, h) -> Option<(x, y)>`.
- `occupied` holds the axis-aligned rects of every tile intersecting the sector, crossing tiles included, each inflated by `padding`.
- The usable area is the sector shrunk by `padding`.
- **Candidates.** x from {inner left} ∪ {occupied right edges}; y from {inner top} ∪ {occupied bottom edges}. The first candidate in top-to-bottom, then left-to-right order that fits inside and overlaps nothing wins. O(n³) in tiles per sector, negligible for hundreds of tiles.
- **Order of sectors.** The active sector first, then the others in UDIM order (`v`, then `u`).
- **Fitting down.** If `w > S - 2p` or `h > S - 2p`, the tile is scaled uniformly by `min((S-2p)/w, (S-2p)/h)` before placement.

Tile sizes round to whole pixels. The fitted size rounds down, so it never exceeds the space.

**Tray.** A tile that fits nowhere goes to the right of the grid's bounding box, with a gap of `S/8`. It reuses the same candidate search over an unbounded region starting at the grid's top.

- *Alternative: a real MaxRects or skyline packer.* Better density, but it assumes it owns all free space. Here users move tiles freely, so the packer must respect arbitrary occupied rects. The candidate search does that and stays deterministic for tests.

### D5. Rendering tiles into exports

For each output image (a sector, or a group sheet), every tile whose rect intersects it is drawn:
1. **Render the tile at its placed size** with the existing `warp::warp`. The source is the original; the source "quad" is the original's rect corners, rotated by `rot` quarter turns; the output size is the tile's `w x h`. At scale 1 and any `rot` this reduces to an exact pixel permutation. Task 2.2 checks that against an index-mapping fast path. When shrinking, the existing Jacobian-adaptive supersampling gives the RIP-quality filtering the spec asks for.
2. **Clip the rendered tile to the output rect** and composite it with premultiplied source-over, in manifest order.

The output starts fully transparent.

- **Group sheet.** It is a sheet of cells. Unselected cells stay transparent, and the 16384 px per side limit keeps a sheet at most 1 GiB in RGBA.
- **SECTOR with one sector** writes `<name>.png`. With several sectors selected it writes one file per sector, named like a UDIM set, so the names never clash.
- **UDIM set.** It renders each selected sector separately and numbers it `1001 + (u - u_min) + 10 * (v - v_min)`.

**Display in the UI.** The `ripper://atlas/<ws>/<tile>?max=2048` endpoint serves each original downscaled to at most 2048 px on its long side. Results are cached in the working folder as `thumbs/`, so the WebGL canvas never uploads 8K images.

### D6. ATLAS UI

- **Mode switch.** A `seg-row` RIP | ATLAS at the top of the side panel, using the O'Tool segmented control. Each mode owns a stage element: the existing `#stage` for RIP, a new `#atlasStage` for ATLAS. Switching hides one and shows the other, so the RIP image and quads stay intact.
- **ATLAS panels:**
  - **Workspace:** name with an unsaved dot; NEW, OPEN, SAVE, SAVE AS; sector size select, disabled when there are tiles; padding.
  - **Sector:** the selected sector's UDIM number; REMOVE, disabled with a reason.
  - **Export:** the selection count; scope SECTOR / GROUP FILE / GROUP SET; base name; EXPORT, which opens the folder picker.
- **Canvas.**
  - WebGL2 draws the tile textures and the sector fills.
  - The overlay canvas draws sector borders, selection and handles.
  - "+" buttons are DOM buttons positioned over the canvas, with an SVG glyph per the icon rule. O'Tool has no plus glyph, so it is Lucide `plus`, as the designer chose (task 5.1).
  - Pan and zoom follow RIP: wheel, Space+drag, middle drag, F to fit.
- **Keys in ATLAS** (by `e.code`, as in 1.0.0):
  - Delete removes the selection; R and Shift+R rotate;
  - Esc clears the selection and never closes the app in ATLAS;
  - Ctrl+S saves, Ctrl+Shift+S saves as, Ctrl+O opens, Ctrl+N creates a new workspace.

  RIP keys are unchanged.
- **Atlas output in RIP.** A third slot in the Output panel: a checkbox, a read-only field with the workspace name, and no browse button, since the workspace is chosen in ATLAS.

### D7. Atlas output in the export command

- `ExportReq` gains nothing. Rust reads `atlas_on` and `current_workspace` from settings, as it already does for folders.
- For Send and Save All, each rendered quad's PNG is also passed to `atlas::add_tile`. That function writes the item, then locks the workspace, places the tile and writes the manifest.
- `ExportResult` gains `atlas: { added: n, tray: n, error: Option<String> }`.
- `exit_after` additionally requires `atlas.error == None` when the atlas is on, so ShareX mode stays open on an atlas failure.
- Clipboard, ShareX output and folders are written before the atlas, so an atlas problem cannot delay or block them.

### D8. Version and release

- `Cargo.toml` becomes 2.0.0.
- `release.ps1` publishes the release once the designer says yes.
- Installed 1.0.0 copies then see the update button. This is the first real end-to-end run of the update path, recorded as the follow-up from the archived installer change.

## Risks / Trade-offs

- **[No undo]** A deleted or misplaced tile cannot be restored with Ctrl+Z. → The `.ripper` file keeps the last saved state, and the status reports every delete with a count. Undo is a candidate for 2.1.0 or later.
- **[A stale lock blocks ShareX adds]** → A lock older than 10 s is broken (D3). An add still waits at most 2 s and then reports a failure, never hangs.
- **[Disk use]** Originals are kept at full size, so a workspace of many 4K captures runs into hundreds of MB. → It lives under `%APPDATA%`, not the Drive-mirrored `E:\Projects`. Purging unreferenced items happens on SAVE: items no tile references are left out of the archive and removed from the working folder.
- **[Polling cost]** → One `stat` and, on change, a small JSON read every 500 ms, only while ATLAS is visible.
- **[Memory for large exports]** → At most one 16384² sheet (1 GiB) at a time. SECTOR and GROUP SET render one sector at a time.
- **[UDIM numbering surprises]** Numbers are relative to the selection, not the canvas. → After an export the status row names the files written, for example `cafe.1001.png ... cafe.1012.png`.

## Migration Plan

- No data migration: 1.0.0 has no atlas.
- `config.toml` gains `atlas_on` (false) and `current_workspace` (empty); missing fields use these defaults through `#[serde(default)]`.
- The first switch to ATLAS, or the first Send with the Atlas output on, creates an Untitled workspace.
- Rollback to 1.0.0 is a reinstall. 1.0.0 ignores the unknown settings fields, and the workspaces stay on disk for a later 2.x.

## Open Questions

- None open. The sector "+" glyph is Lucide `plus` (designer's choice, task 5.1).
