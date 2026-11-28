# M4 Core Archive Workflows — Notes

Status as of M4-CP003 (2026-11-28): **READY FOR REVIEW** (M5 is NOT approved).

Exit criterion (PROJECT_STATUS): *archive browse / edit / import / export / rebuild work through the web.* Verified with synthetic
archives built by the real `tlk-rpf-core` (no real-world `.rpf` was ever supplied) through the real server binary and Chromium.

## What the user can do now
Upload an `.rpf` → browse → create folder, add files or a whole folder (browser chooser), replace a file or a nested archive,
rename (F2), move, delete, edit/create text entries (via the API; the existing text editors call the same commands) → undo/redo →
**Save** (keeps the result in the workspace and backs up the version it replaces) → **Backups** panel (download / restore) →
export any selection as a **ZIP download** → "Download archive" (current state, including unsaved edits) / "Farklı kaydet" (same download).
The uploaded original is never modified and can always be downloaded (`/files/{id}/download?version=original`).

## Design (as built)
| Concern | Decision |
|---|---|
| Edit engine | `web/doc-engine` (`edit.rs`, `entry.rs`, `export.rs`): port of the desktop `Belge` semantics — snapshot before edit, failed edit rolls back without touching history/redo, success clears redo, history cap 60, revision ids restored by undo/redo (`dirty` = current revision ≠ clean revision). Core is called, nothing re-implemented. |
| Save | serialize → re-parse to verify → (backup previous `current`) → write staging, fsync → atomic rename → `mark_saved` (history kept). First save of a file makes no backup: the immutable original already is the previous version. |
| Storage | `files/{id}/original` (immutable), `files/{id}/current` (last save), `files/{id}/backups/{seq}`; backups capped at 20 per file; originals + current + backups all count against the workspace quota; the oldest backups are dropped to make room, a save that cannot fit is refused with `WORKSPACE_QUOTA_EXCEEDED` and changes nothing. Backup ids are looked up, never joined into paths. |
| Concurrency | One mutex per open document; optional `If-Match: <revision>` → `412 STALE_DOCUMENT`. The web transport always sends it, and reloads the document when it is refused (E2E: edit from a second "tab" is refused, view refreshes, retry works). |
| Imports/replace | Multipart straight into the engine (`POST /documents/{id}/import?dir=`, each part's file name is its relative path; `POST …/replace?path=`). Entire request is held in memory, bounded by `MAX_UPLOAD_BYTES`. Relative paths are validated (`..`, empty segments, `\`, absolute, control chars rejected); `.rpf` parts are opened as nested archives, unparsable ones stay plain files; one undo step per import. |
| Export | `POST …/export {paths[]}` → ZIP (deflate). Entry names are sanitized per component (no `..`, `\`, absolute); nested selections are pruned; nested archives are written as re-serialized `.rpf`; size cap = `MAX_UPLOAD_BYTES` → `EXPORT_TOO_LARGE`. `X-Zyn-File-Count` header carries the number of files. |
| Dialogs | Desktop `open()` → browser file chooser; returns `web-pick:<n>` handles, resolved by the transport at command time. "Choose export folder" (recognised by its desktop title) returns `web-download:` and the export arrives as a ZIP download. `save()` (save-as to a path) is still unavailable. |
| Ordering | Commands for one document run in issue order (`src/web/queue.ts`). |
| Superseded reads | The reused file viewer re-reads its entry whenever the UI re-renders (e.g. when the rename dialog closes). That read can land right after the user's own rename/move removed the path. For 2 s after a structural edit, a failed `dosya_oku` with `NOT_FOUND` resolves to `null` (no content) instead of toasting a spurious error. Reads failing with no recent edit still surface. The browser console still logs the 404 (that request is real). |
| Selection | After any edit the shell clears a selection whose path no longer exists. |

## Routes added (deviations from `M1_API_CONTRACT.md` marked ⚠)
`GET entries?path=` (entry view), `GET entries/raw?path=`, `PUT|POST entries/text`, `POST import?dir=` ⚠ (multipart directly; contract said `{dir,fileIds[]}` via prior upload), `POST replace?path=` ⚠ (same), `POST export` (ZIP), `POST delete|rename|mkdir|move|undo|redo|save`, `POST restore {backupId}` ⚠ (per document), `GET /files/{id}/backups` ⚠ and `…/backups/{backupId}/download` ⚠ (per file instead of global `/backups`), `GET /files/{id}/download?version=original|current`.
Error codes added: `STALE_DOCUMENT` 412, `CONFLICT` 409, `INVALID_NAME` 400, `EXPORT_TOO_LARGE` 413, `NOTHING_TO_UNDO|REDO` 409.

## Desktop commands now mapped (13 new; 17 of ~60 in total incl. `ayarlar_oku`)
`dosya_oku, metin_yaz, metin_olustur, sil, yeniden_adlandir, dizin_olustur, tasi, geri_al, yinele, kaydet (no `hedef`), ice_aktar, dosya_degistir, disa_aktar`.
Deliberately **not** mapped: `yedekleri_listele`/`yedek_geri_yukle` (the desktop Backups view is path/mods-folder based; the web shell has its own English Backups panel using the per-file routes), `kaydet` with a target path, `yeni_mod`, `mod_sikistir` (serialization already writes without dead space; the standalone "compress" action does not exist), `icerikte_ara`, `belgeler_arasi_kopyala`, all texture/model/map/GFX/AI/mods commands (M5+).

## Verification (2026-11-28)
- `cd web && cargo test --workspace`: doc-engine 8 unit + 18 edit; server 10 unit + 11 api (M2/M3) + 5 m4 route tests — all pass. `cargo test -p tlk-rpf-core` (core): 16 + 14 pass.
- `node --test web-tests/adapt.test.mjs`: 11 pass (adapters, handle kinds, export-title coupling, KeyedQueue).
- `npx tsc`, `npm run build` (desktop) and `npm run build:web` pass; web bundle has 0 `@tauri-apps` references.
- Chromium E2E (`web-tests/e2e-m4.cjs`; real `zyn-server` binary + `dist-web`), 15 steps, 5/5 runs green: upload+open, create folder, import files (chooser) and a nested folder, replace, view, F2 rename, move, delete+undo+redo+undo, ZIP export (names + content checked with `unzip`), save (dirty→clean, original byte-identical, current differs), undo-after-save dirty, second save creates a backup, Backups panel restore, stale edit refused + refreshed + retried, reload restores state, close-dirty confirmation, hostile import path refused (400 INVALID_PATH). The run asserts the exact set of failed requests and that the only error toast is the deliberate stale-edit one.
- Desktop sources (`core/`, `src-tauri/`, `src/` outside `src/web/`, `package.json`, `Cargo.lock`) are byte-identical to the M3-CP002 package.

## Known limitations / not verified
- **No real `.rpf` was ever tested**; only synthetic archives. Encrypted (AES/NG) archives, very large archives and nested real DLC packs are untested. The "real-file round-trip" checklist item stays open.
- Whole archives live in memory (open document = archive bytes + undo snapshots up to 60 deep); memory use is unmeasured. Railway RAM limits are unknown. Import/replace buffers the request body in memory.
- Backups/current are lost when the workspace expires or the server restarts (workspace index is in memory; Railway disk is ephemeral). Users must download what they want to keep.
- Two browsers sharing one cookie share a workspace; concurrent edits are serialized, stale ones refused (412), but there is no merge.
- UI text of reused components is still Turkish until M7 (e.g. "Kaydet", prompts, confirm dialogs). One coupling: the "export" dialog is recognised by the Turkish title `"Nereye çıkarılsın?"`; a Node test pins it so M7 cannot silently break it.
- "Show in folder", save-as-to-path, folder drops onto tree targets, keyboard shortcuts (Ctrl+S/Z) in the web shell, search-in-contents and new-empty-mod are not implemented.
- No rate limiting (M8); no SPA CSP; no Dockerfile; `tauri build` not run.
