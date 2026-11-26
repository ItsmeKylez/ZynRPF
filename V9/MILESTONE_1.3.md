# Zyn-RPF Web — Milestone 1.3: Standard RPF7 editing and rebuilding

## Implemented
- Added `buildRpf()` to write an RPF7 archive with directory records, filename table, 512-byte payload alignment and standard uncompressed file records.
- Added archive workspace operations for adding standard files, renaming standard files and deleting standard files from the in-memory rebuild set.
- Added repack/download with a parser round-trip check before download.
- Added archive path validation, duplicate/case-collision rejection, directory/file collision checks, entry-count and filename-offset limits.
- Added a helper to extract all supported standard entries for the edit workspace.
- Existing standard DEFLATE extraction remains supported.

## Important safety limits
- Repacking is blocked when the source archive contains RSC7 resource entries. The current writer cannot preserve resource page metadata and must not silently drop those entries.
- AES/NG encrypted archives remain unsupported.
- Writer currently emits uncompressed ordinary files and limits each payload to less than 16 MiB. It does not claim compatibility with every RPF7 variant or GTA V archive.
- Nested RPF archives are currently treated as ordinary file payloads by the writer; nested archive browsing/editing is not implemented.
- The UI's add operation adds files at archive root. Rename/delete currently apply to standard file entries only.

## Verification
- `node --check src.js` — passed.
- `node --check rpf-engine.js` — passed.
- `node tests/rpf-engine.test.mjs` — passed: header/table parsing, directory tree, plain extraction, invalid magic, archive creation, nested directory output, extraction round-trip, path traversal rejection, duplicate path rejection and compressed-entry extraction.
- `node build.mjs` — passed; static `dist/` generated.
- Real GTA V archive compatibility and browser end-to-end interactions — not verified in this environment.

## Not complete
RSC7 page mapping/decompression and resource-preserving write-back, nested RPF recursion/editing, compression preservation, AES/NG decryption, YTD, YPT, model/map/GFX editors, backend/Railway deployment and browser E2E tests remain pending.
