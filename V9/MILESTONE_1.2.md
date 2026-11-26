# Zyn-RPF Web — Milestone 1.2

## Delivered
- Added a browser-side RPF7 archive table parser based on the original `core/src/rpf.rs` format layout.
- Validates RPF7 magic, record count, name-table bounds, root directory, child ranges, duplicate references, and maximum directory depth.
- Displays archive metadata, encryption marker, directory/file paths, entry types, and stored sizes.
- Supports extraction of ordinary non-resource file entries. Compressed ordinary entries are decoded with browser `DecompressionStream('deflate-raw')` where supported.
- Provides explicit errors for AES/NG encrypted archives and marks RSC7 resources as raw-only pending resource decompression and virtual-page mapping.
- Includes synthetic fixture tests for header parsing, directory tree construction, plain extraction, and invalid magic.

## Not delivered / not to be represented as complete
- RPF archive write-back/repacking, adding/removing/renaming entries.
- RSC7 resource decompression and virtual page mapping.
- YTD parser/writer integration, YPT parser/writer integration, model/map/GFX format editing.
- Browser runtime end-to-end testing in a real browser.

## Verification
- `node --check src.js` — passed.
- `node --check rpf-engine.js` — passed.
- `node tests/rpf-engine.test.mjs` — passed.
- `node build.mjs` — passed; generated `dist/`.

## Implementation note
The parser follows the record layout in the included original Rust `core/src/rpf.rs`. It is a focused browser implementation of RPF7 table reading, not a claim that the full Rust archive engine has been ported. Keep the original Rust implementation as the authoritative reference for edge cases and write-back semantics.
