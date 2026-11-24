# M2 REPORT: Web UI (2026-11-24)
Phase M2 approved by the user (reply `APPROVED` to M2_PROPOSAL.md). Framework left open by the user; React 18 + Vite 5 + TypeScript chosen, as the proposal preferred when the registry is reachable (it was).

## Done (VERIFIED = observed in a run in this Linux container, Chromium via Playwright)
- Web UI in `web/`: automatic anonymous session (kept across page reload), storage and retention notice, "Delete my data".
- Upload: file picker and drop zone, chunked upload with progress, pause/resume, automatic retry of transient chunk failures, resume after hard failure, optional SHA-256 check (files up to 256 MiB), parse-job progress, API errors shown with message, next step, code and request ID.
- Workspace and archive selector, delete workspace.
- Browser: breadcrumb, up, virtualised sortable grid (name, kind, size) with keyboard support (arrows, Page, Home, End, Enter), search with results list, entry detail panel (path, kind, detected type, sizes, offset, flags).
- Read-only hex viewer (4 KiB pages, previous/next, go to offset), entry byte download (up to 64 MiB), whole-archive download.
- Server: serves the built UI from `WEB_DIR` (files loaded at startup, no request path reaches the file system), cache headers, `/me` now returns the CSRF token so a reload keeps the session, default read rate limit raised from 120 to 600 per minute (UI paging needs it; value is SUGGESTED, unmeasured).
- Tests, all passing:
  - 41 browser checks: boot, upload, browse, keyboard navigation, sort, hex content equals the file bytes, entry download equals the archive slice, search, archive download equals the original, reload keeps the session, cross-session isolation, delete-my-data removes files on disk, transient-failure retry, resume after hard failure, pause and resume, hostile (truncated) upload error display, 1,000,000-entry archive (upload and parse 4.4 s through the browser, End key renders in 333 ms with 22 DOM rows), no CSP violations or page errors.
  - axe-core (WCAG 2.0/2.1 A and AA rules) found no violations in the empty, archive, detail/hex and error states. It found one real issue early (unlabelled file input); fixed.
  - English audit script: non-ASCII scan, catalogue coverage both ways, literal-text scan, UK spelling list; passes, and was checked to catch an injected literal.
  - M1 regression: 70 API checks and rpf-core tests still pass.

## Not done / not verified
- Real game archives (acceptance N, D-035). Compressed-entry bytes, RSC7 and texture preview are not supported (UI says so).
- Browsers other than Chromium; manual screen-reader and keyboard-only review (axe finds only part of WCAG 2.1 AA); colour-contrast beyond axe's check; phone layout (non-goal, D-018).
- Dockerfile gained a web build stage but is still unbuilt. TLS/proxy, Railway (M4), load testing.
- npm dependency licence check (D-028 covers Rust only so far); runtime deps are React and ReactDOM (MIT), build tools Vite (MIT) and TypeScript (Apache-2.0); axe-core (MPL-2.0) is test-only and outside the repo.
- Spell check is a small UK-spelling list, not a dictionary.

## Deviations and notes
- D-039 deviations unchanged (std HTTP, in-memory state, in-process parse, polling, Linux-only).
- Backend touched: static serving, `/me` CSRF field, one default rate limit. No other API change.
- Upload resume after reload works only while the server session and upload still exist (stored in the browser's localStorage); a server restart wipes everything (D-039 item 2).
- The grid pages 500 rows per request and keeps loaded pages in browser memory; very long scrolling sessions on 1M-entry folders are untested beyond the End-key case.

## Risks
Layout facts still PARTIAL, so a real archive may show surprising values. Read rate limit is global per session, not per IP. Single-stream upload speed over real networks is unmeasured.

## Next
Phase M3 (edit and save, with validation) needs your explicit approval. Nothing further has been started.
