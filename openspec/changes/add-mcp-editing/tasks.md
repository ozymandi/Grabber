# Tasks

## 1. Second binary and packaging

- [x] 1.1 Spike (design D1): add a `[[bin]] ripper-mcp` with `src/bin/ripper-mcp.rs` that prints its version to stdout, and run `cargo tauri build`. Verify:
  - the NSIS setup installs `%LOCALAPPDATA%\ripper\ripper-mcp.exe` next to `ripper.exe`;
  - the exe answers on stdout when it is started with piped stdio;
  - uninstall removes it.

  Do this on a test install with `%APPDATA%\ripper` backed up via Bash, and stop for the designer if the bundling needs another route.
- [x] 1.2 Make `scripts/release.ps1` refuse to publish a setup without `ripper-mcp.exe` (the bundler already builds and installs every binary, D1). Verify with `release.ps1 -DryRun` that the check passes and the setup lists `ripper-mcp.exe`.

## 2. Model: tile history, sector log, format 3

- [x] 2.1 Add `PictureState`, `Tile::history/current`, `SectorEntry` and `Manifest::sector_log` (design D4). Make `parse_manifest` migrate format 2 to a one-state history and bump `FORMAT` to 3, and make `Manifest::items()` include every state's item. Verify with `cargo test`:
  - a 2.0 and a 2.3 manifest open with "Original" as the only state;
  - a format 4 file is refused;
  - SAVE AS keeps and packs every state's item and drops the rest;
  - undo of a Delete restores the history.
- [x] 2.2 Add `Workspace::set_picture` (keep_alpha, filter, name, by) and `go_to_state` for tiles and sector entries, under the lock (D6, D4). Verify with `cargo test`:
  - a 64 × 64 opaque picture on a 512 × 512 Circle tile keeps the circle's alpha resampled;
  - `keep_alpha: false` keeps the picture's own alpha;
  - going to a lower state keeps the states above;
  - going to a sector entry skips a deleted tile and names it;
  - a concurrent `add_tile` loses nothing (spec "Capture during an edit").
- [x] 2.3 Add `filter = nearest` to `render_tile` / `render_frame` (D6). Verify with `cargo test` that a 64 × 64 state rendered at 512 × 512 gives exact 8 × 8 blocks, and that `smooth` output is byte-identical to 2.3.0's.
- [x] 2.4 Add sector render to a PNG with tile rects, and `write_sector` cutting back by frames, skipping tilted and border-crossing tiles (D6). Verify with `cargo test`:
  - a sector with three upright tiles, one turned by 90°, written back unchanged, gives each tile a state equal to its frame-size render;
  - a tilted tile and one crossing the edge are named and untouched;
  - one `SectorEntry` lists the three tiles.

## 3. MCP server

- [x] 3.1 Implement the stdio JSON-RPC loop (D2): `initialize` with `instructions`, `notifications/initialized`, `ping`, `tools/list` and `tools/call`, with errors as JSON-RPC errors and logs on stderr. Verify with an integration test that drives the binary through a recorded transcript and checks every reply.
- [x] 3.2 Implement the tools of D3 on `ripper_core`, with temp files under `%TEMP%\ripper-mcp\<id>\`. Verify with integration tests on a temp workspace root:
  - `atlas_info` lists tiles;
  - `get_tile` writes a full-size PNG and a ≤256 px thumbnail;
  - `write_tile` adds a state;
  - `render_sector` + `write_sector` round-trip;
  - `go_to_state` works;
  - `propose` writes `proposal.json`.
- [x] 3.3 Scope resolution (D5): read `ui-state.json` with the 5 s heartbeat, resolve explicit target → tiles → sectors → atlas (`confirm: true`), and freeze the result under `scope_id`. Verify with `cargo test` for every scenario of spec "Scope follows the selection", including a stale file and "Selection changes mid-task".
- [x] 3.4 Write the README section "Connect an LLM" with the server path and config for Claude Code, Codex and Gemini CLI. Verify by connecting Claude Code to the built server with the same command through a one-off `--mcp-config` on a test home and listing the atlas (done: "MCP check: 3 tiles, sectors 1001"); the documented `claude mcp add` against the installed exe is checked in 6.1.

## 4. ATLAS window: selection, live pictures, proposals

- [x] 4.1 Publish `ui-state.json` from ATLAS on selection change and every 2 s while visible (D5). Verify via CDP that the file follows a marquee selection and a sector click, and goes stale 5 s after the window closes.
- [x] 4.2 Show the current picture state of every tile, with `nearest` states drawn without smoothing on the canvas. Verify via CDP: a `write_tile` from the server appears within 1 s (spec "Open window updates"), and a pixel-art state shows hard pixels at 400 % zoom.
- [x] 4.3 Proposal preview with APPLY / DISCARD (D7): ghost frames, crossed-out deletes, dashed new sectors, and a bar with the text. APPLY is one undo step with conflicts named; DISCARD removes the file. Verify via CDP for spec "Proposed move", "Discard" and "Undo an applied proposal".
- [x] 4.4 Add to the help overlay and README: History, Ctrl+C / Ctrl+V of states, and proposals. Verify by reading both against the specs.

## 5. History panel

- [x] 5.1 Build the History panel at the top left of the ATLAS canvas area (D8): rows with thumbnails, name, author and time, newest on top, the current state marked, and the hint text for other selections. Verify via CDP for spec "Select a tile" and the hint with two tiles selected. Show a screenshot to the designer.
- [x] 5.2 Click a row to go to that state, for tiles and sector entries, including the "Before …" row. Verify via CDP for spec "Back to the original" and "Undo a sector restyle by history", and that Ctrl+Z after a click reports nothing to undo.
- [x] 5.3 Ctrl+C on a row and Ctrl+V in ATLAS with the `PasteStates` edit: a tile state goes in by automatic placement, a sector entry keeps its layout, and each paste is one undo step. Verify with `cargo test` for the edit and via CDP for spec "Paste an older state", "Undo a paste" and "Paste a pixelized sector".
- [x] 5.4 Designer review in a test build. Claude Code is connected to `ripper-mcp.exe` and works on a copy of real tiles, with `%APPDATA%\ripper` backed up via Bash and restored afterwards; «Building» is never the current workspace. Check:
  - pixelize the selection;
  - restyle a sector both ways;
  - History and paste;
  - a proposal.

  Record the outcome in `task.md`. Verify with the designer's explicit approval.

## 6. Integration and release

- [x] 6.1 End-to-end on a release build with a test workspace. Use Claude Code over MCP and another client (Codex or Gemini CLI, whichever is installed):
  - scope by selection;
  - write tiles with the alpha kept;
  - whole-sector write;
  - a proposal applied;
  - save as `.ripper`, reopen, histories intact;
  - export pixel-for-pixel equal to the current states.

  Verify the results, and that a Rect tile with no LLM states exports byte-identical to 2.3.0.
- [ ] 6.2 Bump to 2.4.0 and run `release.ps1 -DryRun`. Publish only after the designer's explicit yes, then update this machine through the button. Verify that the installed `ripper-mcp.exe` reports 2.4.0 after the update.
- [ ] 6.3 Update `task.md` (status, next step) and the project memory. Verify that both name the 2.4.0 release.

## Workflow follow-up

- Archive with `/opsx:archive add-mcp-editing` after the release. This creates the main specs `atlas-mcp` and `atlas-history` and updates `atlas-tiles`, `atlas-undo`, `atlas-workspace` and `app-distribution`.
