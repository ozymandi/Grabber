# Proposal: add-mcp-editing

## Why

Restyling ripped textures (pixelize, change the style, recolour) today means exporting tiles, editing them in another program and bringing them back by hand. LLM clients such as Claude Code, Codex and Gemini CLI can already do such edits with their own image tools, skills (ComfyUI) or other applications; ripper only needs to let them read and write the atlas safely, under the designer's control.

## What Changes

- **MCP server for the atlas.** A separate `ripper-mcp.exe` (stdio) installed next to `ripper.exe`. An LLM client connects to it and gets tools to read the current workspace (sectors, tiles, thumbnails, full-size PNG paths, sector renders) and to write back new tile pictures. ripper does no image editing itself: the LLM uses its own means.
- **Scope follows the selection.** The open ATLAS window publishes its selection. Selected tiles are the target; else selected sectors; else the whole atlas, after the LLM asks. An explicit target named in the request wins. The scope is frozen when the request starts.
- **New tile pictures are versions.** Writing a picture keeps the tile's frame, adds a version on top of the tile's history and reapplies the original alpha (the quad's shape mask) by default. Resolution may change; the frame stays.
- **Sector and atlas edits, two ways.** Per tile, or as one sector render that the LLM edits whole and ripper cuts back into the tiles by their frames. The request chooses.
- **The LLM proposes, the designer decides.** Picture replacements are agreed in the chat and then written. Layout changes (move, scale, rotate, delete, sectors, export) can only be proposed: ATLAS shows a proposal as a preview with APPLY and DISCARD.
- **History panel in ATLAS.** A stack of picture states of the selected tile or sector, top-left of the canvas area. Clicking a state makes it current. A state can be copied and pasted into the active sector as an ordinary new object. History is not undo and does not cover the workspace itself.
- Workspace format 3 stores tile histories; ripper 2.3 and older refuse format 3 files instead of dropping the histories.

## Capabilities

### New Capabilities
- `atlas-mcp`: the MCP server: connecting a client, reading the atlas, scope by selection, writing tile pictures, sector edits per tile or whole, and proposals for layout changes.
- `atlas-history`: per-tile and per-sector picture history: what is recorded, the History panel, going to a state, and copying a state into the atlas as a new object.

### Modified Capabilities
- `atlas-tiles`: "Tile keeps its original" — a tile renders from its current picture state; the original stays in its history.
- `atlas-undo`: "What enters the history" — APPLY of a proposal and a paste are steps; picture changes by the LLM and going to a history state are not.
- `atlas-workspace`: "Workspace document" — the `.ripper` file also holds every tile's picture history.
- `app-distribution`: "Per-user installer" — the installer also places `ripper-mcp.exe` next to `ripper.exe`.

## Impact

- **Rust:** new `src-tauri/src/bin/ripper-mcp.rs` (stdio JSON-RPC, MCP tools) on `ripper_core`; `atlas/model.rs` (tile `history`, `Manifest::items` includes versions, FORMAT 3, proposals), `atlas/store.rs` (set picture, go to state, write/read proposal and UI state files), `atlas/render.rs` (render a sector to a file, cut a sector image back into tiles), `atlas/edit.rs` (paste from a state).
- **UI:** `ui/js/atlas/` — History panel (new), publishing the selection, proposal preview with APPLY / DISCARD, Ctrl+C / Ctrl+V on history rows; `ui/index.html`, `ui/css/app.css`.
- **Packaging:** `tauri.conf.json` bundles the second binary; `scripts/release.ps1` unchanged otherwise.
- **Docs:** README section on connecting Claude Code, Codex and Gemini CLI.
- **No network** is added to ripper; the server talks only to its client over stdio and to the workspace on disk.
