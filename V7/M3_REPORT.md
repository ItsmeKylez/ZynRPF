# M3 REPORT: Edit and save (built 2026-11-24)
Status: built and tested in this container. Awaiting your review. M4 (Railway) is NOT started and needs its own approval.

## How M3 was approved (please confirm)
Your `APPROVED` replies came after the recovery plan and after my message asking for an approval that names M3. I treated them as approval of M3 exactly as written in M3_PROPOSAL.md, with defaults for its three questions: stored bytes only, D-041 accepted with D-042 open, no extra operations. If you meant something narrower, tell me and I will revert.

## What was built
- **rpf-core** (`rebuild.rs`, `validate.rs`, new codes in `error.rs`): five logged operations (rename, delete, new folder, add file, replace contents), a layout planner, and a streaming writer. Untouched payloads are copied byte-for-byte from the source file (D-006). The names table is regenerated within limits (D-008). 16 new check codes, appended only (D-031). Standalone validator with severities (Error blocks publish).
- **server** (`api/edit.rs`): per-workspace operation log with undo/redo, a one-writer lease (D-020), content upload (`PUT /workspaces/{id}/blobs`), dry-run check, Save as a background job (rebuild to a temp file, re-parse and validate, then publish; any failure publishes nothing and removes the temp file), version cap 10, version delete, standalone validate of any version, storage quota checked before the rebuild.
- **web**: `EditPanel` (toolbar, pending-change list with undo/redo, Check, Save with progress, validation report, version list with download, check and delete), read-only mode for a second tab. All text is in the catalogue.

## Verified in this container
| Check | Result |
|---|---|
| `cargo test --offline` | 4 earlier + 7 new rpf-core tests pass: round trip, all five ops, untouched bytes identical, hostile names/duplicates/limits/depth/names-table, compressed and large-resource entries copied verbatim, validator duplicate and overlap detection, 400 random edit sequences (no panic; every accepted result validates clean) |
| M3 API end-to-end (`scripts/e2e-m3.mjs`) | 78 of 78 pass, including interrupted save (missing content: nothing published, temp file removed, pending changes kept), version cap, quota, writer lock, isolation, cleanup |
| M1 API regression (`scripts/e2e.mjs`) | 70 of 70 pass |
| M3 browser test (`scripts/ui-test-m3.cjs`, Chromium) | 23 of 23 pass, incl. two tabs (second is read-only, takes over after the first closes), save, download of the new version, no CSP errors |
| English audit | passes (171 catalogue keys) |

## NOT verified
- **Real game archives and in-game behaviour (acceptance N): not verified (D-035).** Alignment, entry order, and name-table rules come from the same PARTIAL format notes. A rebuilt file that re-parses here may still fail in the game.
- **UI type check and build:** `tsc` and `vite` could not run (no network, no `@types/react`). The UI was bundled with a local esbuild and React 19.2.8 (package.json pins 18.3.1). Please run `npm ci && npm run build` on your side.
- **axe accessibility scan and the M2 browser suite** (axe-core is not installed here). The new panel uses labelled controls, `role=status/alert`, and keyboard-reachable buttons, but it has had no axe scan or screen-reader review.
- Other browsers, TLS/proxy, Docker, Railway, load, very large archives with edits (a 1M-entry replay was not timed).
- Fuzzing of the rebuild path is 400 sequences, not the CI-scale fuzzing in D-010.

## Known limits and behaviour you should know
1. **The browser keeps showing the version you opened until you Save.** Pending changes are listed, not previewed in the tree.
2. Edit state and uploaded content live in memory and on disk per workspace; a server restart loses pending changes (D-039 still applies).
3. Added and replaced files are stored uncompressed. Resource entries (RSC7) cannot be replaced; they can be renamed, deleted and copied.
4. Existing entries keep their original order inside a folder; new entries go last. Entries not reachable from the root are dropped on save.
5. A large-resource entry's length is taken as the distance to the next payload (padding included).
6. Rename and delete work on folders; there is no move or copy.

## New decisions needing your confirmation (D-044, proposed)
- Default write rate limit raised 30 to 120 per minute (the writer heartbeat and edits would hit 30).
- New API: `PUT /workspaces/{id}/blobs` (limit `MAX_BLOB_BYTES`, default 256 MiB), `/edit`, `/edit/ops`, `/edit/undo|redo|discard|validate|save`, `/writer`, `DELETE /versions/{id}`, `POST /versions/{id}/validate`. `/limits` gains `max_blob_bytes`, `max_ops` (1000), `max_versions` (10). Jobs gain `type` and `report`.
- Writer lease: 30 s, renewed every 10 s by the open tab, released when the page closes.
- Early rejections of a request body drain up to 8 MiB so the client receives the error instead of a connection reset.

## Run it
```
cargo run -q --release -p rpf-core --example gen -- small.rpf 5 50000
PORT=18081 DATA_DIR=./data SESSION_QUOTA_BYTES=3000000 MAX_BLOB_BYTES=1048576 RATE_READS_PER_MIN=2000 RATE_WRITES_PER_MIN=2000 CHUNK_SIZE_BYTES=1048576 ./target/release/server
node scripts/e2e-m3.mjs 18081 ./data small.rpf
```
For `scripts/e2e.mjs` use `gen good.rpf 5 6291456` and `gen big.rpf 30000 1000`.
