# Zyn-RPF Web — Milestone 1.4

## Implemented in this increment
- RPF archive loading now retains RSC7 resource payloads as raw bytes instead of silently omitting them from the rebuild workspace.
- RPF rebuild passes through resource payload bytes and the resource system/graphics size metadata.
- Repack is no longer blocked solely because an archive contains RSC7 entries.
- RSC7 resource rename/delete operations remain blocked in the UI until the resource-specific semantics are understood and tested.
- Added an **Open nested** action for `.rpf` file entries. The nested payload is extracted and parsed as its own archive workspace.
- Added automated tests for resource byte/metadata round-trip and nested RPF extraction/parsing.

## Verification
- `node --check src.js` — passed.
- `node --check rpf-engine.js` — passed.
- `node tests/rpf-engine.test.mjs` — passed, including standard extraction, compressed extraction, writer round-trip, path traversal rejection, resource passthrough and nested archive parsing.
- `npm run build` — passed; static `dist/` generated.

## Limitations
- Resource passthrough is not RSC7 decompression, virtual-page mapping or editing. It only preserves raw resource bytes and metadata through rebuild.
- Real GTA V archive fixtures have not been supplied/verified in this environment.
- AES/NG encryption is unsupported.
- The JavaScript writer still has a 16 MiB per-entry limit and a 64 KiB filename-offset table limit.
- The original Rust core contains the authoritative implementations for RPF, RSC7, YTD, YPT, models, maps and GFX, but this environment has no Rust toolchain, so a Railway Rust API wrapper has not yet been compiled or verified.
- The project is not yet a complete deployable replacement. Production backend, Railway deployment, persistent user storage, authentication/session isolation and original feature parity remain outstanding.
