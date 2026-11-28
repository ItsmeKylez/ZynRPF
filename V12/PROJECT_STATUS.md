# Project Status — Zyn-RPF Web Conversion

> This file is the persistent progress record across Claude accounts. Update it after every meaningful work session. Never mark work complete without evidence.

## Approval Gate

- **Current stage:** M5 — Editors. Sub-milestone **M5a (visual settings) READY FOR REVIEW**
- **Current stage status:** M5a READY FOR REVIEW (M5a-CP001, 2026-11-28). M5 started 2026-11-28 after the user answered "M4 accepted — start M5" (2026-11-28 21:37 +07:00). Further families (M5b texture editor, YPT, model/collision, map/radar, GFX, meta) are NOT approved yet.
- **User approval received for this stage:** YES — M5 start approved (answer "M4 accepted — start M5", 2026-11-28 +07:00). Earlier: M4 start ("Yes, start M4"), M3 ("I approve M3" 2026-11-27).
- **npm / frontend dependency install (`npm ci`): APPROVED by user 2026-11-27 (answered the question; ran once, lockfile unchanged).** No other new packages approved.
- **D3 (new Rust crates + network fetch for `web/` only): APPROVED by user 2026-11-27 ~10:20 +07:00** (no npm; desktop lockfiles untouched). D4/D6: user kept defaults.
- **Previous stages:** M0 (M0-CP001), M1 (M1-CP001), M2 (M2-CP005) delivered. D1, D2, D5 were not answered explicitly: work proceeded on the recommended defaults (D1=A, D2=English DTOs, D5=env keys). D3 (Rust crates for `web/`) and `npm ci` were approved (see above); D4 limits and D6 licence kept at defaults. No other dependencies are approved.
- **Next stage:** M6 — Security/reliability
- **Permission to begin next stage (M6):** NO. Within M5 each editor family is delivered as its own READY FOR REVIEW ZIP and the next family needs explicit approval (FEATURE_CHECKLIST Q).

**Rule:** Every milestone requires a separate explicit user approval. A completed ZIP is for review, not permission to continue. See `APPROVAL_GATE.md`.

## Overall status

- **Current milestone:** M4 — Core archive workflows (READY FOR REVIEW; M5 NOT approved)
- **Overall completion:** M0 + M1 documents delivered; M2 batches 1 and 2a written. Web crates NOT compiled with real deps (no crate cache); only std-only parts verified with plain `rustc`.
- **Current state (M5a-CP001):** visual-settings family done: engine `settings.rs`, routes, `vs_oku`/`vs_ayarla` mapped (19 of ~60 desktop commands), existing editor works in the browser; Chromium E2E 8 steps ×3 + M4 E2E 15 steps ×3 (after fixing weak waits in the harness). Details, deliberate differences (value validation, unknown key = error) and limits: `docs/M5A_NOTES.md`. M4: `docs/M4_NOTES.md`.
- **Last updated:** 2026-11-28 (M5a-CP001)
- **Current blocker:** None. Open items for the user: real `.rpf`/`visualsettings.dat` samples (none supplied), Railway RAM/disk limits, D6 licence.
- **Next exact action:** STOP and wait for the user's review. Continue only after explicit approval of the next M5 family (suggested order: M5b texture editor). If the user requests M5a changes, do only those.

## Milestones

| Milestone | Status | Exit criteria |
|---|---|---|
| M0 Source audit | `[~]` audit written (`docs/M0_SOURCE_AUDIT.md`); baseline blocked | Evidence-based inventory, command mapping, baseline tests/build recorded |
| M1 Architecture | `[~]` `docs/M1_ARCHITECTURE.md` + `docs/M1_API_CONTRACT.md` written; awaiting user decisions D1–D6 | Web boundary/API/workspace design documented |
| M2 Vertical slice | `[x]` READY FOR REVIEW (not approved): upload → core opens RPF → tree → entry/export/download works; 24 cargo tests + curl + headless-Chromium run pass; see `docs/M2_SLICE_NOTES.md` | Upload → existing core opens RPF → tree shown → export/download |
| M3 Frontend migration | `[x]` READY FOR REVIEW (not approved) | Typed HTTP adapter and web shell work — verified in Chromium; 4/~60 commands mapped, rest explicit-unavailable |
| M4 Core archive workflows | `[x]` ACCEPTED 2026-11-28: edit/import/replace/export ZIP/save/backups/undo-redo through web — see `docs/M4_NOTES.md` | Archive browse/edit/import/export/rebuild work through web |
| M5 Editors | `[~]` IN PROGRESS: M5a visual settings READY FOR REVIEW; M5b+ not started/approved | Feature families migrated and verified individually |
| M6 Security/reliability | `[ ]` | Workspace isolation, validation, safe writes, backups and limits tested |
| M7 English/UI | `[ ]` | User-facing interface/docs fully English |
| M8 Deployment | `[ ]` | Railway-ready configuration and docs |
| M9 Regression/final | `[ ]` | Tests, limitations, README and handoff finalized |

## Completed work

- [x] Added Claude handoff/foundation documentation to the source package.
- [ ] No application code conversion completed yet.

## Current implementation notes

- Keep `core/` as the binary processing engine.
- `src/api.ts` currently imports Tauri `invoke`; the frontend is not web-ready yet.
- `src-tauri/src/lib.rs` contains command logic that must be mapped to portable core operations versus desktop-only operations.
- Do not remove the Tauri application as part of the first web milestone.

## Test/build baseline

Not yet run. Do not infer that tests pass.

| Command | Result | Notes |
|---|---|---|
| `cargo test -p tlk-rpf-core --offline` | FAILED to resolve deps (not a test failure) | `no matching package named bcdec_rs found` — no registry cache; nothing installed. rustc/cargo 1.97.0 present |
| Existing frontend build script (`tsc && vite build`) | Not run | No `node_modules`; node v22.23.5 present; install not approved |
| `rustc --edition 2024 --test web/server/src/lib.rs` + run | PASSED 8/8 (config 2, store 6), no warnings | std-only; no cargo/deps involved. 2026-11-27 |
| `rustc --test` of `web/doc-engine/src/paths.rs` with stub `EngineError` | PASSED 2/2 | stub replaces `error.rs` (real one needs `tlk-rpf-core`); rest of doc-engine NOT compiled |
| `cd web && cargo test --workspace` | PASSED 24: doc-engine 6, server unit 8, server routes 10 (CP005 count) | 2026-11-27, debug, Linux. Real core + synthetic archives |
| Desktop frontend baseline: `npx tsc` + `npx vite build` (before any M3 change) | PASSED, built in ~0.7 s | first-ever frontend baseline; unchanged after M3 |
| `node --test web-tests/adapt.test.mjs` | PASSED 6/6 | pure mappers, plain Node |
| `npm run build:web` | OK; 0 Tauri runtime refs in `dist-web` (desktop `dist`: 2) | |
| `cargo test --workspace` (web/) after STATIC_DIR | PASSED 25 (doc-engine 6, server unit 8, routes 11) | |
| Drag-and-drop upload in Chromium: `.rpf` drop opens + renders; non-`.rpf` drop rejected with message; no errors | PASSED | synthetic events via `DataTransfer` (not a native OS drag) |
| FINAL (M3-CP002) re-run on final tree: node 6/6, desktop tsc+build, `build:web`, cargo 25/25, `cargo build --release` OK (47 s); existing sources identical to M2-CP001 | PASSED | |
| Playwright/Chromium: real `WebApp` + real server (`STATIC_DIR`): upload, tree via existing `Belge`, XSS-inert name, Download archive + Save-as downloads, unavailable-feature toast, reload restore, SPA deep link, JSON API 404; repeated 10x | PASSED 10/10 (11 API calls each, 1 workspace each). BEFORE the fix: request storm in 2 of 6 runs | drag-drop upload NOT tested |
| Headless Chromium (Playwright) against the real binary: choose file, upload, open, tree, entry + export download, close | PASSED, no console errors; hostile HTML in an entry name rendered as inert text | 2026-11-27 |
| `cd web && cargo build --release -p zyn-server` | OK, 1m40s, 2.0 MB binary; release binary opened a synthetic RPF | resolves the M1 `intel_tex_2`/ISPC risk for Linux (needs `-lstdc++`) |
| Manual: run `target/debug/zyn-server` (PORT=18080) + `curl` upload/open/entry/export/download/CSRF/isolation | All behaved as designed; export byte-identical to input for the synthetic archive | 2026-11-27 |
| `cd web && cargo test -p zyn-doc-engine` (with `web/.cargo/config.toml`) | PASSED 6/6 (document 4, paths 2) | real `tlk-rpf-core`, synthetic archives. Without `-lstdc++` link FAILS (`__gxx_personality_v0`, intel_tex_2 ISPC objs). 2026-11-27 |
| `cargo test -p tlk-rpf-core` in scratch copy (core + web/Cargo.lock, no repo change) | PASSED 16 unit + 14 integration, 0 failed | FIRST core baseline. Debug profile, Linux x86_64, web lock versions (not the desktop lock). Release build / desktop lock NOT tested |
| `cargo metadata --offline` in `web/` | FAILED earlier: workspace member `server` missing (now exists); still no crate cache | `~/.cargo/registry` empty |
| Existing Tauri build | Not run | Not required for web conversion baseline |

## Decisions

- Web frontend: reuse React + TypeScript + Vite.
- Processing: Rust backend reusing existing `core/`.
- Hosting target: Railway.
- Source control: GitHub.
- Browser file access: explicit upload/download and server-side workspace; no silent local folder access.
- Dependency installation: requires explicit user approval.

## Changed files

M3-CP002 (2026-11-27): docs only (stage close-out); no code change since CP001.

M3-CP001 (2026-11-27): added `src/web/*` (wire, adapt, http, transport, unavailable, WebApp, main, shims/*), `web.html`, `vite.web.config.ts`, `web-tests/adapt.test.mjs`, `docs/M3_NOTES.md`; `package.json` scripts only (dev:web, build:web, test:web); `.gitignore` (+dist-web/). Server: `STATIC_DIR` (config.rs), SPA fallback + API JSON 404 (app.rs), +1 test, `tower-http` (fs) added. No existing frontend/core/Tauri file changed.

M2-CP005 (2026-11-27): added `web/server/static/{dev.html,dev.js}`; edited `web/server/src/app.rs` (serves `/`, `/dev.js` with strict CSP), `web/server/tests/api.rs` (+1 test), `web/server/examples/make_sample_rpf.rs` (optional `hostile`). Docs updated.

M2-CP004 (2026-11-27): added `web/server/src/app.rs`, `web/server/src/main.rs`, `web/server/tests/api.rs`, `web/server/examples/make_sample_rpf.rs`, `docs/M2_SLICE_NOTES.md`; edited `web/server/{Cargo.toml,src/config.rs,src/store.rs,src/lib.rs}` (cookie_secure, ready(), sweep returns tokens). `web/Cargo.lock` re-resolved (axum/tokio/tokio-util + deps; serde_json 1.0.151→1.0.154). Nothing outside `web/` and control docs changed (verified by diff vs M2-CP001).

M2-CP003 (2026-11-27): added `web/.cargo/config.toml`; `web/Cargo.lock` re-resolved by cargo (95 packages, existing versions kept; adds zyn-doc-engine/zyn-server). Docs updated. No source changes.

M2-CP002 (2026-11-27): added `web/server/Cargo.toml` (no deps), `web/server/src/{lib,config,store}.rs`; edited status/checklist/handoff/log. `doc-engine` unchanged.

M1 (2026-11-26): added `docs/M1_ARCHITECTURE.md`, `docs/M1_API_CONTRACT.md`; edited status/checklist/handoff/log. No code.

M0 (2026-11-26): added `docs/M0_SOURCE_AUDIT.md`; edited `PROJECT_STATUS.md`, `FEATURE_CHECKLIST.md`, `HANDOFF.md`, `CHECKPOINT_LOG.md`. No source code changed.

Foundation documents:

Only the added foundation documents are currently expected:
- `CLAUDE_START_HERE.md`
- `CLAUDE_MASTER_PROMPT.md`
- `WEB_CONVERSION_BASELINE.md`
- `FEATURE_CHECKLIST.md`
- `PROJECT_STATUS.md`
- `HANDOFF.md`

Update this list after code changes.

## Known risks / unresolved questions

- Exact Tauri command registration and parameter contracts need source audit.
- Determine which core operations currently depend on path-based IO or Tauri-managed state.
- Determine how image/map/model preview URLs are currently served and adapt safely.
- Determine whether existing AI provider handling can be extracted without exposing secrets.
- Verify feature claims against actual source and tests.


## Checkpoint delivery rule

Follow `CHECKPOINT_AND_USAGE_LIMIT_PROTOCOL.md`: send an initial full ZIP before substantial work, then a verified checkpoint ZIP after every meaningful batch and immediately when a usage-limit warning appears. Update `CHECKPOINT_LOG.md` for every delivered checkpoint. Interim checkpoints do not authorize the next stage.
