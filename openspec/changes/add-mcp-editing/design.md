# Design: add-mcp-editing

## Context

See proposal.md for why. What the code already gives:

- A workspace is a working folder `%APPDATA%\ripper\workspaces\<id>\` with `manifest.json` and immutable `items/*.png`. Every write is lock → re-read → apply → atomic write (`atlas/store.rs`), so a ShareX process and an open window already write side by side.
- The ATLAS window polls `atlas_revision` every 500 ms and reloads on change (`ui/js/atlas/atlas.js`), so writes from another process show up without new plumbing.
- A tile is `item` + `w0/h0` (the full-size picture) and a `Frame` (centre, size, quarter turns, tilt, pivot). Rendering always goes from the full-size picture through `render_frame` (`atlas/render.rs`). `Manifest::items()` decides which item files survive a SAVE AS.
- ATLAS undo (`atlas-undo`) is window-local; edits from elsewhere are kept and conflicts are skipped.
- `ripper.exe` is a GUI-subsystem binary in release builds; `ripper_core` is a Tauri-free library.

## Goals / Non-Goals

**Goals:**
- A client can read the atlas, write tile pictures and propose layout changes without the GUI running.
- Every write is safe next to an open window and ShareX captures.
- History and proposals live in the workspace, so they survive restarts and travel in `.ripper` files.

**Non-Goals:**
- No image editing, model calls or network in ripper; the client brings its own tools.
- No MCP over HTTP or for remote machines.
- No LLM access to RIP mode (photos and quads).
- No per-request undo of LLM pictures beyond History.

## Decisions

### D1. A separate console binary `ripper-mcp.exe`

A second `[[bin]]` in the same crate, `src/bin/ripper-mcp.rs`, console subsystem, depending only on `ripper_core`; `default-run = "ripper"` keeps `cargo run` on the app. The Tauri bundler installs every binary of the package, so the NSIS setup and the updater carry `ripper-mcp.exe` next to `ripper.exe` and the uninstaller removes it, with no `externalBin` (spike 1.1: the generated `installer.nsi` has the `File` and `Delete` lines, the setup holds one `ripper-mcp.exe` that answers over piped stdio).

*Alternatives:* `ripper.exe --mcp` — rejected: a GUI-subsystem exe has no console, and relying on inherited pipe handles in a GUI binary is fragile. `bundle.externalBin` — rejected after the spike: it duplicated the binary in the setup and needs the file before the package's own build runs.

### D2. Hand-written MCP over stdio

Newline-delimited JSON-RPC 2.0 on stdin/stdout: `initialize` (protocol `2025-06-18`, server `instructions`), `notifications/initialized`, `ping`, `tools/list`, `tools/call`. Logs go to stderr. Tool results carry text (JSON) and, where useful, one `image` content item (the thumbnail, base64 PNG).

*Alternative:* the `rmcp` crate. Rejected for now: an async runtime and a larger dependency for eight tools; the protocol subset is small and is pinned by a transcript test (task 3.1).

The `instructions` tell the client the working rules from the proposal: confirm before a whole-atlas scope, agree pictures in the chat with thumbnails, use proposals for layout.

### D3. Tools

| Tool | Does |
|---|---|
| `atlas_info` | Workspace name, sector size, sectors, active sector, tiles (id, sector, frame, picture size, provenance, history length, current state). |
| `get_scope` | Resolves the scope (D5); returns tile ids, sectors, `source` (`request`, `tiles`, `sectors`, `atlas`), `confirm` (true for the whole atlas). |
| `get_tile` | Writes the current full-size picture to a temp PNG, returns its path and a ≤256 px thumbnail. |
| `render_sector` | Renders a sector at full size (transparent fill) to a temp PNG; returns the path and each tile's rect in it, marking tiles that cannot be written back whole. |
| `write_tile` | New picture from a PNG path: `name`, `keep_alpha` (default true), `filter` (`smooth` / `nearest`). |
| `write_sector` | A sector render from a PNG path, cut back into its tiles (D6). |
| `go_to_state` | Makes state N of a tile, or entry N of a sector, current. |
| `propose` | Writes a proposal of layout edits (D7). |

Temp files go to `%TEMP%\ripper-mcp\<workspace id>\`. Paths written by the client are read as given.

### D4. Tile history in the manifest; format 3

`Tile` gains `history: Vec<PictureState>` and `current: usize`. A `PictureState` holds `item`, `w0`, `h0`, `name`, `by`, `at` and `filter`. The tile's existing `item/w0/h0` mirror the current state, so placement and the renderer stay unchanged. `Manifest` gains `sector_log: Vec<SectorEntry>`: sector, name, by, at, and for each changed tile its state index before and after.

- `parse_manifest` turns a format 2 tile into a history of one state, named "Original", by "RIP".
- `FORMAT` becomes 3, so ripper 2.3 refuses such files instead of silently dropping histories on its next write.
- `Manifest::items()` includes every state's item, so SAVE AS keeps and packs them.
- Deleting a tile drops its states with it. Undo of a delete restores the tile, history included, because `RestoreTile` carries the whole tile.

Going to a sector entry sets each listed tile that still exists to the entry's "after" index. The bottom row of a sector's list, "Before <first entry>", uses that entry's "before" indices.

*Alternative:* versions in a side file. Rejected: two files would have to stay consistent under the lock, and `.ripper` packing would need a second path.

### D5. Scope from a published UI state

While ATLAS is visible, the window writes `ui-state.json` in the workspace folder: selected tile ids, selected sectors, a timestamp and the window's process id. It writes on every selection change, and at least every 2 s as a heartbeat. The server treats a file older than 5 s as "no window".

Resolution order: explicit target in the call, then selected tiles, then selected sectors (all their tiles), then the whole atlas with `confirm: true`. `get_scope` returns a `scope_id`; later calls that pass it use that frozen list.

*Alternative:* the server asks the running app over a local socket. Rejected: needs the GUI to run a server, and fails when no window is open, which D1 allows.

### D6. Writing pictures, alpha and whole sectors

`Workspace::set_picture(tile, png, name, by, keep_alpha, filter)` decodes the PNG and, with `keep_alpha`, multiplies its alpha by the original's alpha resampled to the new size. That is the bottom state's picture, so a quad's shape mask survives. It then adds the item, pushes a state and makes it current, all under one lock.

`write_sector` loads the PNG (it must match the rendered size), then for each tile with tilt 0 lying wholly inside the sector:
1. crop the tile's rect;
2. undo its quarter turns with `rotate_exact`;
3. store the crop at frame size as the new picture, with alpha kept from the original.

Tilted tiles and tiles crossing the sector edge are skipped and named. One `SectorEntry` records the request.

`filter = nearest` makes `render_tile` and the ATLAS canvas scale that state without smoothing, for pixel art. The default `smooth` keeps today's path.

### D7. Proposals as a file, applied by the window

`propose` writes `proposal.json` in the workspace folder: the list of `Edit`s (the same enum ATLAS undo uses: `Move`, `Scale`, `Rotate`, `Turn`, `Delete`, `AddSectors`, `RemoveSectors`, `SetGuide`…), a text, `by`, a time and the base revision.

The window shows it on its next poll:
- moved or scaled tiles as ghost frames at their new place;
- deleted tiles crossed out;
- new sectors dashed;
- a bar with the text and APPLY / DISCARD.

APPLY runs `apply_steps` under one lock and records the inverse as one ATLAS undo step. Parts that no longer apply are skipped and named, like undo conflicts. DISCARD, or a newer proposal, deletes the file. Export is not an `Edit` and is left out of proposals for now; the client can describe it in the text.

### D8. History panel and copy / paste

The panel is a floating panel at the top left of `#atlasStage`, in the free canvas area shown in the designer's screenshot. It lists rows with thumbnails served by the existing `ripper://atlas/<ws>/<item>?max=N` protocol.

- Clicking a row calls a new `atlas_go_to_state` command.
- Ctrl+C on the focused panel puts the state in an in-app clipboard (not the OS clipboard).
- Ctrl+V in ATLAS, unused today, pastes it: a new `PasteStates` edit adds tiles through `place_new`, or keeps the relative layout for a sector entry. It is recorded as one undo step whose inverse is a `Delete`.

### D9. Version 2.4.0

The release carries both binaries. The README gets a section with the server path and the config snippets for Claude Code (`claude mcp add`), Codex (`config.toml`) and Gemini CLI (`settings.json`).

## Risks / Trade-offs

- [A second binary might not land in the install folder on a real update.] → The spike checked the setup's script and contents; task 6.2 checks the installed folder after a real update.
- [An 8192² sector render is ~256 MB raw and slow to write.] → Render only on request; the reply gives the size first; large renders go to `%TEMP%` and are deleted when the server starts again.
- [Keeping the original alpha can clash with a restyle that changes the silhouette.] → `keep_alpha: false` per call; the History panel reverts any state.
- [`ui-state.json` can be stale if a window crashes.] → 5 s heartbeat rule; the scope reply says which source it used.
- [A proposal built on an old revision.] → Applied best effort with skipped parts named, the same model as ATLAS undo conflicts.
- [Format 3 locks out 2.3 and older.] → Everyone updates through the button; the refusal message already names the cause.
- [Clients differ in MCP support (images in results, instructions).] → Every result also has text, and paths work without image support.

## Migration Plan

Format 2 workspaces open with a one-state history and are written as format 3 on their first change. To roll back, reinstall 2.3; format 3 workspaces then refuse to open until 2.4 is back, and nothing is lost.

## Open Questions

- Exact thumbnail size of History rows (48 or 64 px): settled in the designer review.
