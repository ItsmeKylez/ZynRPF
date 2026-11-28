# Feature Checklist — Living Acceptance Tracker

**Rule:** Mark `[x]` only after implementation AND verification. A screen existing is not enough. `[~]` means partial/in progress. `[!]` means blocked or requires investigation. Add evidence/test notes beside completed items.

## A. Project foundation
- [~] Source audit completed; core/frontend/Tauri responsibilities mapped. (M0: all 60 Tauri commands, `api.ts`, `kasa.rs`, protocol URIs read and mapped in `docs/M0_SOURCE_AUDIT.md`; other `core/src` modules and React components inspected by grep/signature only — not line-by-line. Awaiting user review.)
- [~] Existing tests and current build status recorded. (UPDATE 2026-11-27, D3 approved: core 16+14 tests PASS in a scratch copy (debug, Linux, web lock); frontend build and Tauri build NOT run. Previously BLOCKED: `cargo test --offline` fails at dependency resolution — no crate cache; no `node_modules`. No test result exists. Needs user approval to fetch deps. See audit §8.)
- [~] Web architecture documented and approved by implementation evidence. (M1 design docs written: `docs/M1_ARCHITECTURE.md`, `docs/M1_API_CONTRACT.md`. NOT yet validated by implementation — M2 must prove it. Open decisions D1–D6.)
- [~] Rust web backend created and calls existing `core/` crate. (`web/server` axum app opens RPFs through `doc-engine` → `tlk-rpf-core`; 23 tests + curl run pass. Only the M2 slice routes exist.)
- [~] Typed frontend HTTP adapter replaces Tauri IPC for web build. (M3-CP001: Vite-alias shims + `src/web/transport.ts`; M4: 19 of ~60 commands mapped (13 archive-workflow commands in M4, `vs_oku`/`vs_ayarla` in M5a), the rest reject explicitly; web bundle contains 0 Tauri runtime refs. See `docs/M3_NOTES.md`, `docs/M4_NOTES.md`)
- [~] Existing Tauri desktop app remains intact/buildable where feasible. (no existing source/manifest/lockfile changed, verified by diff; desktop `tsc` + `vite build` pass; `tauri build`/app NOT run)

## B. Web workspace and file lifecycle
- [~] Single-file upload. (multipart, streamed, size-capped; verified by tests and curl; no browser UI yet)
- [~] Multi-file upload. (server loops over multipart parts; only single-part tested)
- [ ] Folder upload where supported, with documented fallback.
- [~] Drag and drop. (browser file drop onto the web shell uploads+opens `.rpf`, rejects other types; tested with synthetic DataTransfer events in Chromium; folder drop / drop onto specific tree targets (desktop `data-birak` targets) NOT implemented)
- [~] Workspace/session isolation. (store-level only: `web/server/src/store.rs`, test `sessions_are_isolated` passes via rustc; no HTTP/cookie layer yet)
- [ ] Safe workspace file listing and tree navigation.
- [~] Upload size/type validation. (size cap, quota, file-count limits tested in store; NO file-type/magic check yet; no HTTP body limit)
- [~] Path traversal protection. (display-name sanitizer + server-generated ids tested; virtual-path validator `doc-engine/paths.rs` tested via stub; archive-entry/zip-slip checks not yet)
- [x] Download individual file. (original, saved `current`, backups, and single archive entries download; route tests + Chromium E2E; synthetic archives)
- [x] Export selected files/folders as ZIP. (ZIP route + UI; names/content checked with `unzip` in E2E; hostile names sanitized and size-capped in tests; synthetic archives)
- [~] Workspace cleanup/expiry behavior documented and implemented. (idle/max-age sweep + delete implemented and tested in store; no background janitor task yet; not documented for users)
- [ ] Progress/error handling for large uploads and exports.

## C. RPF archive
- [~] Open supported RPF archive through existing Rust core. (doc-engine `Document` opens synthetic archives built by core, tests pass; no real-world .rpf yet; no HTTP route yet)
- [~] Display archive tree and metadata. (JSON tree + summary from API verified; no UI)
- [~] Extract files. (single entry download via API verified)
- [x] Replace file. (engine + route + UI via browser chooser, incl. nested archive; tests + E2E; synthetic archives)
- [x] Import files/folders. (files and a folder via chooser, nested `.rpf` auto-opened, hostile paths refused; tests + E2E; synthetic archives)
- [x] Delete/rename/move/create directory. (engine + routes + UI; self-move guard; E2E; synthetic archives)
- [x] Rebuild/save archive. (Save = serialize → verify → backup → atomic write to workspace; download of saved/current state; E2E; synthetic archives)
- [x] Validate rebuilt archive before download. (every save, export and "Download archive" re-parses the rebuilt bytes; tests)
- [~] Correctly report encrypted/unsupported archive formats. (garbage → 422 RPF_INVALID/CORRUPT verified; real AES/NG/encrypted archives untested)
- [ ] Real-file round-trip test evidence recorded.

## D. Mods and assembly
- [ ] Upload/manage mod packages.
- [ ] Parse assembly mapping.
- [ ] Report missing/invalid sources.
- [ ] Detect conflicts.
- [ ] Compare conflicting files/settings/textures.
- [ ] Enable/disable behavior represented as workspace state (not local FiveM install).
- [ ] Explain that direct installation into local FiveM folders is unavailable from ordinary web browser.

## E. Texture editor
- [ ] List YTD textures.
- [ ] Preview textures.
- [ ] Replace texture.
- [ ] Add texture.
- [ ] Delete texture.
- [ ] Rename texture.
- [ ] Resize texture.
- [ ] Recolour texture.
- [ ] Export one texture.
- [ ] Batch export.
- [ ] Batch import.
- [ ] Preserve supported formats/compression/mipmaps.
- [ ] Verify embedded texture workflows for `.ydr`, `.ydd`, `.yft`, `.ypt`.

## F. Visual settings
- [x] Read and display grouped settings. (core parser via `vs_oku`; existing grouped editor in Chromium E2E; synthetic file)
- [x] Edit scalar values. (text field, float `f` suffix kept, all duplicates updated, one undo step; engine + route + E2E)
- [x] Colour picker workflows. (picker rewrites 3 channel keys, 0–255 scale checked in E2E; 0–1 scale parsed by the core, not E2E'd)
- [~] Presets. (use the same `vs_ayarla` path; preset UI not exercised in the browser)
- [x] Preserve comments, line order and duplicate keys. (byte-exact assertions incl. CRLF, engine tests + E2E)
- [x] Save and re-open verification. (save → reload → reopen shows the edited values; original upload byte-identical; E2E)

## G. Particle/YPT
- [ ] Read YPT effects/emitters/particle rules.
- [ ] Display editable properties and keyframes.
- [ ] Edit values and keyframes.
- [ ] Search.
- [ ] Batch colour/size/light/density operations.
- [ ] Undo/redo integration.
- [ ] Save and re-open verification.

## H. 3D model and collision
- [ ] Model summary/list.
- [ ] Render supported model formats.
- [ ] Render collision meshes where supported.
- [ ] Select components and inspect textures.
- [ ] Preview texture edits.
- [ ] Export OBJ and textures.
- [ ] Empty-model detection.
- [ ] Verify large-model performance and error handling.

## I. Map/radar editor
- [ ] Read map layout and tiles.
- [ ] Display map/radar tiles.
- [ ] Add image/text/logo layers.
- [ ] Position/scale/rotate/opacity/blend controls.
- [ ] Coordinate mapping preserved.
- [ ] Bake affected tiles.
- [ ] Undo/restore layers after bake.
- [ ] Save and re-open verification.

## J. Scaleform/GFX
- [ ] Open supported GFX.
- [ ] List embedded images.
- [ ] Preview embedded images.
- [ ] Replace image.
- [ ] Export image.
- [ ] Display external texture references.
- [ ] Save and re-open verification.

## K. Meta files
- [ ] Read supported `.ymt`.
- [ ] Read supported `.ymap`.
- [ ] Read supported `.ytyp`.
- [ ] XML display/edit.
- [ ] Binary write-back.
- [ ] Validation/warnings.
- [ ] Round-trip tests.

## L. Backup, history, safety
- [x] Backup before every destructive save. (previous saved version backed up, last 20 kept; the first save needs none because the original upload is immutable; tests + E2E)
- [x] Atomic write/export flow. (staging + fsync + rename; failed saves leave `current` intact; store tests)
- [x] Undo. (engine + route + UI, history cap 60, E2E)
- [x] Redo. (engine + route + UI, E2E)
- [x] Restore backup. (validated before it replaces the live version; forged ids → 404; route test + E2E)
- [x] Preserve original upload until explicit export/replace. (original never written; byte-identical check in route test + E2E)
- [x] Validate after mutation. (save/restore/export re-parse; replace/import parse nested archives; invalid input rolled back)
- [x] Handle concurrent/stale document state safely. (per-document lock, `If-Match` revision → 412, UI reloads; route test + E2E second-tab scenario)

## M. AI assistant
- [ ] Existing provider architecture audited.
- [ ] Server-side provider key storage/configuration.
- [ ] No API secrets returned to browser.
- [ ] Supported providers and compatible endpoint behavior verified.
- [ ] AI tool operations limited to authorized workspace/document.
- [ ] Clear error handling and opt-in network disclosure.

## N. English/UI
- [ ] All user-facing Turkish strings translated.
- [ ] Errors/dialogs/tooltips/settings translated.
- [ ] Help/docs updated to web behavior.
- [ ] No broken labels or mixed-language interface.
- [ ] Keyboard shortcuts adapted to browser-safe behavior.
- [ ] Responsive layout and accessible interaction basics verified.

## O. Railway and production
- [x] Backend binds to `0.0.0.0`. (verified: server log `listening on 0.0.0.0:<port>`, curl via 127.0.0.1)
- [x] Uses Railway `PORT`. (verified: PORT=18080/18081/18082 runs; invalid PORT exits with error)
- [x] Health endpoint. (`/healthz`, `/readyz` tested)
- [~] Production frontend serving/routing configured. (server serves `dist-web` via `STATIC_DIR` with SPA fallback + JSON API 404, route-tested; no Dockerfile/Railway run)
- [ ] Environment variable documentation.
- [ ] Upload/workspace limits documented.
- [ ] Temporary storage lifecycle documented.
- [ ] Secrets excluded from repository/frontend.
- [ ] Railway deployment instructions tested or clearly marked unverified.

## P. Final verification
- [ ] Existing core tests pass (record exact command/output).
- [ ] New backend tests pass.
- [~] Frontend build passes. (Linux: desktop `tsc`+`vite build` and `npm run build:web` pass; 2026-11-27)
- [x] Upload → open RPF → inspect → edit → export vertical slice verified. (Chromium E2E 15 steps ×5 runs, synthetic archive only)
- [ ] Regression test matrix completed.
- [ ] Known limitations documented.
- [ ] README reflects actual web app behavior.
- [ ] `PROJECT_STATUS.md` final state accurate.
- [ ] `HANDOFF.md` final state accurate.


## Q. Approval gates (mandatory)
- [x] M0 source audit ZIP delivered and user explicitly approves M1. (M0-CP001 delivered; user: "approve M1" 2026-11-26 19:52 +07:00)
- [x] M1 architecture ZIP delivered and user explicitly approves M2. (user: "approve M2" 2026-11-26 20:00 +07:00)
- [x] M2 vertical slice ZIP delivered and user explicitly approves M3. (M2-CP005 delivered; user: "I approve M3" 2026-11-27 10:23 +07:00)
- [x] M3 frontend migration ZIP delivered and user explicitly approves M4. (M3-CP002 delivered; user: "APPROVED" 2026-11-28 03:46, then "Yes, start M4")
- [x] M4 archive workflows ZIP delivered and user explicitly approves M5. (M4-CP003 delivered; user "APPROVED" + answer "M4 accepted — start M5", 2026-11-28 21:37 +07:00)
- [ ] M5 editor milestone(s) delivered separately and each next stage explicitly approved. (M5a visual settings: READY FOR REVIEW, M5a-CP001; other families not started)
- [ ] M6 security/reliability ZIP delivered and user explicitly approves M7.
- [ ] M7 English/UI ZIP delivered and user explicitly approves M8.
- [ ] M8 deployment ZIP delivered and user explicitly approves M9.
- [ ] M9 final ZIP delivered and final review completed.

**Important:** Claude may only check an approval item after explicit user approval. Preparing or delivering a ZIP does not count as approval.
