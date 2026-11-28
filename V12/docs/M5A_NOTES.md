# M5a — Visual settings editor — Notes

Status as of M5a-CP001 (2026-11-28): **READY FOR REVIEW**. M5 is split into editor-family sub-milestones, each delivered and
approved separately (FEATURE_CHECKLIST §Q). **M5b (next family) is NOT approved.**

## Scope
Family F of the checklist: `visualsettings.dat` read / edit / colour pickers / revert / save. Desktop commands `vs_oku` and `vs_ayarla`
are mapped; the existing React editor (`AyarEditoru`) runs unchanged on top of them.

## Design
- `web/doc-engine/src/settings.rs`: `Document::settings_view` and `settings_apply` call the core's `vs::Ayarlar` (parse / `ayarla` / `metin`); no format logic was re-implemented, so comments, line order, spacing, CRLF, duplicate keys and the float `f` suffix are preserved by the same code the desktop uses.
- One batch = one undo step (as on the desktop, where the UI debounces edits into one batch).
- Routes: `GET /documents/{id}/settings?path=` → `{entries:[{key,value,line,repeat}], colors:[{base,keys[3],scale}]}`; `PUT /documents/{id}/settings {path, values:[[key,value]]}` → document summary (supports `If-Match`).
- Frontend: `vs_oku` / `vs_ayarla` in `src/web/transport.ts`, mapper `toSettings` in `src/web/adapt.ts` (Node-tested).

## Deliberate differences from the desktop (both prevent silent corruption)
1. **Values are validated**: a value must be a single token (no whitespace, no line breaks, ≤256 chars; keys likewise). The desktop passes the string straight into the file, so a value containing a line break would inject new lines and an emptied input would delete the value (turning the line into a comment-like key without value, which then disappears from the editor). The web version answers `400 INVALID_REQUEST` and changes nothing.
2. **Unknown keys are an error** (`404 NOT_FOUND`) instead of a no-op reported as success. The whole batch is validated before anything is applied (a bad entry never leaves earlier entries applied).

## Verification (2026-11-28)
- Engine tests `web/doc-engine/tests/settings.rs` (4): entries/duplicates/colours; apply preserves `#`/`//` comments, CRLF, float suffix, all duplicates, one undo step, undo restores the exact bytes; refused batches (unknown key, newline injection, spaces, empty, empty batch, valid-then-invalid mix) change nothing and keep revision and history; missing/hostile paths.
- Route test (`web/server/tests/m4.rs::visual_settings_read_and_update`): read, update, exact resulting bytes, error codes, cross-session isolation.
- Chromium E2E `web-tests/e2e-m5a-settings.cjs` (8 steps, 3/3 runs): open settings file in the existing editor ("5 ayar · 1 renk" from the core parser); edit a duplicated key through the text field (both lines change, comments + CRLF intact, one undo step, undo restores exact bytes); float suffix kept (`2.25` → `2.25f`); colour picker rewrites the three channel keys (0–255 scale); invalid value refused with a clear toast and unchanged file; "revert all" restores opening values; save → reload → reopen persists and the original upload is byte-identical; settings call on a missing file → 404. Asserts the exact set of failed requests and the only error toast.
- Totals: cargo — core 16+14, doc-engine 8+18+4, server 10+11+6; Node 12; `tsc`, desktop build and web build pass; web bundle has 0 `@tauri-apps` refs. Desktop sources (`core/`, `src-tauri/`, `src/` outside `src/web/`, `package.json`, `Cargo.lock`) are byte-identical to the M3-CP002 package.

## Test-harness fix recorded
While writing this E2E I found that Playwright's `waitForFunction(async …)` does not wait for the async result, so six waits in the M4 E2E were effectively no-ops. All waits in both scripts now use a polling helper (`until`). The M4 E2E was rerun with the corrected waits: 15/15 steps, 3/3 runs.

## Known limitations
- Only synthetic `visualsettings.dat` content was tested; no real-world file was supplied. The core parser's own tests are the evidence for real-file formats.
- Presets (`HazirPanel`, which calls the same `vs_ayarla`) were not exercised in the browser; they go through the identical path and the same validation. Marked `[~]`.
- Typing a value with spaces in the existing UI shows the server's English error (the surrounding UI text is still Turkish until M7).
- Same global limitations as M4 (in-memory documents, no rate limiting, Turkish UI strings, ephemeral Railway disk).
