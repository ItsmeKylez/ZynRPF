# Zyn-RPF Web Conversion — Build Status

## Target
Convert the original Zyn-RPF/TLK-RPF desktop tool into a browser-accessible application while preserving its capabilities and using the uploaded source as the canonical reference.

## Current delivered state (Stage 0 — Web workspace prototype)
- Responsive workspace shell and navigation.
- Browser file upload and session file list.
- Standalone image preview/export/replacement.
- Browser text editor for supported text files with download-copy save.
- Dashboard and module placeholder screens.

**Not complete:** binary-format operations are not implemented by this prototype. UI scaffolds are not counted as feature-complete.

## Required stages to reach complete web edition

### Stage 1 — Web architecture and file session model
- Replace ad hoc state with typed file/session state and persistent IndexedDB storage.
- Establish API contract, upload/download streaming, operation queue, job status, errors, logs.
- Authentication/authorization if hosted multi-user deployment is planned.
- Keep original files immutable and record edit operations.

### Stage 2 — RPF/CFXP engine
- Audit core/src/rpf.rs and related RSC code for WASM compatibility.
- Port parser, list, extract, add, replace, delete, rename, repack, metadata and integrity checks.
- Support only formats/versions verified by tests; surface unsupported/encrypted archives clearly.
- Build fixtures and regression tests from original test corpus.

### Stage 3 — Texture dictionary (YTD)
- Port YTD reader/writer and texture resource mapping.
- Implement list/preview, export, import, replace, rename, resize, recolor, mipmaps and format controls where original supports them.
- Round-trip test DDS/compression modes.

### Stage 4 — Visual settings / config editors
- Preserve comments, duplicate keys, ordering, line endings and unknown entries.
- Add grouped settings UI, typed controls, presets, diff/undo/redo, validation and export.

### Stage 5 — YPT particle editor
- Port parser and serializer for effect/emitter/particle data and keyframes.
- Search, filter, bulk operations, color/size/light/rate edits, preview when supported.
- Round-trip fixtures and no-op preservation tests.

### Stage 6 — Models and collision
- Port supported model/collision parsers.
- Three.js viewer, camera controls, mesh/material/texture selection, diagnostics, export supported by original.
- Explicitly label non-editable mesh operations if source does not support authoring.

### Stage 7 — Map editor
- Rebuild interactive map/tile/layer editor and coordinate transforms.
- Layers for images, logos, text; rotate/scale/place, bake, restore layers, undo.
- Verify tile-specific encoding/write-back.

### Stage 8 — GFX / Scaleform
- Port supported extraction, asset browsing, replacement, export and referenced-texture inspection.
- Preserve unsupported structures and reject unsafe writes.

### Stage 9 — Mods manager, conflicts, package mapping
- Mods listing/enable-disable, assembly mapping validation/repair, conflict detection and comparison.
- Explain why an archive won't load; install workflow only when user provides target directory or backend permission.

### Stage 10 — AI assistant, backup, history and UX
- Provider-agnostic AI configuration; never expose server secrets in browser.
- AI tool calls constrained to the same validated edit APIs, confirmation for destructive operations.
- Backup before writes, operation history, undo/redo, recent files, command palette, search, keyboard shortcuts.

### Stage 11 — Deployment, hardening, verification
- Separate frontend and backend build/deploy for Railway (or compatible platform).
- Upload limits, rate limits, malware/zip-bomb protection, path traversal defense, cleanup/retention.
- Unit tests, fixtures, browser tests, integration tests, end-to-end archive round trips.
- Document hosting, env vars, local dev, limitations and migration.

## Engineering rule
A feature is complete only after its real data path, write-back, error states, tests, and user-visible behavior are implemented. A button or screen alone is not completion.

## Stage 1.1 — Browser workspace improvements (current milestone)

Implemented in the current worktree:
- Persistent browser-local upload vault using IndexedDB (reopens saved uploads on next visit).
- Drag-and-drop import from the desktop into the web workspace.
- Standalone image lab with canvas preview, 90-degree rotation, horizontal/vertical flips, brightness, contrast, saturation and edited PNG export.
- Read-only hex/ASCII inspector with 512-byte paging for arbitrary binary uploads.
- Edited text files and replaced images are written to the browser vault; source uploads remain download-safe.
- Module UI clearly labels standalone image processing as distinct from YTD dictionary processing.

Verified: `node --check src.js` passes; `node build.mjs` produces the static `dist/` build.
Not verified: browser interaction tests (headless launch timed out in this environment), RPF/YTD/YPT/GFX/model/map binary processing, backend API, Railway deployment.

## Engine port audit notes

The original Rust core has separate modules: `rpf.rs`, `rsc.rs`, `ytd.rs`, `doku.rs`, `ypt.rs`, `gfx.rs`, `harita.rs`, `vs.rs`, `model.rs`, `modlar.rs`, and `oiv.rs`. The source is retained under `reference/original-source`. A Rust toolchain is not installed in this build environment, so no WASM compilation is claimed. Next engine milestone should establish a backend Rust build first, then expose safe typed operations to the web UI; avoid claiming browser WASM support before dependency/toolchain validation.

## Stage 2.1 — Standard RPF7 editing and rebuild (Milestone 1.3)
Implemented:
- Browser-side RPF7 writer for uncompressed ordinary files with directory tree generation and 512-byte alignment.
- RPF workspace add-file, rename-file, delete-file and repack/download actions.
- Repack runs parser round-trip validation before download.
- Repack is blocked for archives containing RSC7 resources to prevent silent resource loss.
- Path traversal, duplicate paths, file/directory collisions and size/offset limits are validated.

Verification: `node --check src.js`, `node --check rpf-engine.js`, `node tests/rpf-engine.test.mjs`, and `node build.mjs` pass. Tests cover writer round-trip, compressed extraction and invalid path cases.

Limitations: ordinary file payloads only; writer emits uncompressed data and currently limits each file below 16 MiB. Resource entries, AES/NG encryption, nested RPF editing, real GTA V compatibility, browser E2E and Railway deployment are not verified/implemented. See `MILESTONE_1.3.md`.
