# Checkpoint Log

This file records every recoverable ZIP delivered to the user.

## Latest checkpoint

- Checkpoint ID: M5a-CP001 — **READY FOR REVIEW** (M5a visual settings, 2026-11-28)
- Stage: M5 — Editors, sub-milestone M5a. M5 start approved ("M4 accepted — start M5", 2026-11-28 21:37 +07:00). Next family: NOT approved.
- Included: `web/doc-engine/src/settings.rs` + `tests/settings.rs`; settings routes in `web/server/src/app.rs` + route test; `vs_oku`/`vs_ayarla` in `src/web/{wire,adapt,http,transport}.ts`; `web-tests/e2e-m5a-settings.cjs`; E2E harness fix (`until` polling) in `web-tests/e2e-m4.cjs`; `docs/M5A_NOTES.md`; updated status/checklist/handoff.
- Tests: core 16+14; doc-engine 8+18+4; server 10+11+6; Node 12; `tsc`, desktop + web builds; E2E settings 8 steps ×3 and M4 15 steps ×3, all green. Desktop sources byte-identical to M3-CP002.
- Known gaps: synthetic files only; presets UI not exercised; Turkish UI strings until M7; see `docs/M5A_NOTES.md`.
- Next action: wait for the user's review / approval of the next family.

- Checkpoint ID: M4-CP003 — **READY FOR REVIEW** (M4 stage package, 2026-11-28)
- Stage: M4 — Core archive workflows. Approval to START M4: yes. Approval to begin M5: **NO**.
- Included: everything in M4-CP002 plus frontend: `src/web/{http,transport,adapt,wire,queue,Backups,WebApp}.ts(x)`, `src/web/shims/{dialog,picks}.ts`, `web-tests/adapt.test.mjs`, server export file-count header, `docs/M4_NOTES.md`, updated status/checklist/handoff.
- Tests: core 16+14; doc-engine 8+18; server 10+11+5; Node 11; `tsc`, desktop build, web build; Chromium E2E 15 steps, 5/5 runs green. Desktop sources byte-identical to M3-CP002.
- Known gaps: synthetic archives only; memory unmeasured; Turkish UI text of reused components (M7); see `docs/M4_NOTES.md`.
- Next action: wait for the user's review and explicit M5 approval.

- Checkpoint ID: M4-CP002 — **IN-PROGRESS CHECKPOINT — NOT A STAGE RELEASE** (2026-11-28)
- Stage: M4 IN PROGRESS (approved); M5 NOT approved.
- Included: store `current`/backups/quota (`web/server/src/store.rs`), M4 routes (`app.rs`), `tests/m4.rs`, `zip` dev-dependency (server tests only; already approved D3 crate).
- Tests: `cargo test --workspace` in `web/` all pass (doc-engine 8+18, server 10 unit + 11 api + 5 m4).
- Not done: frontend transport/dialog mappings, browser E2E, README/docs.
- Next action: M4 batch 3 (frontend).

- Checkpoint ID: M4-CP001 — **IN-PROGRESS CHECKPOINT — NOT A STAGE RELEASE** (2026-11-28)
- Stage: M4 IN PROGRESS (approved); M5 NOT approved.
- Included: M4 batch 1 engine: `web/doc-engine/src/{edit,entry,export,paths,document}.rs`, `tests/edit.rs`.
- Tests: `cargo test --workspace` in `web/` — doc-engine 8 unit + 18 edit tests, server 8 unit + 11 route tests, all pass.
- Not done: HTTP routes, store backups/save, adapter/dialog mappings, browser E2E (batch 2).
- Next action: M4 batch 2.

- Checkpoint ID: M4-CP000 — **IN-PROGRESS CHECKPOINT — NOT A STAGE RELEASE** (M4 start snapshot)
- Date: 2026-11-28. M3 approved ("APPROVED") and M4 start explicitly confirmed. Contents = M3-CP002 + approval-state edits only. No M4 code yet.

## Previous checkpoint (M3-CP002)

- Checkpoint ID: M3-CP002 — **READY FOR REVIEW** (M3 frontend migration). Not approved; M4 NOT started.
- Date: 2026-11-27 (Asia/Phnom_Penh). Contents = M3-CP001 code + drag-drop verification + stage close-out docs.
- Tests (final tree): cargo workspace 25/25; Node adapter 6/6; desktop `tsc` + `vite build`; `npm run build:web` (0 Tauri refs); release server build; Chromium E2E 10/10 + drag-drop; existing sources byte-identical to M2-CP001.
- Known gaps: `docs/M3_NOTES.md` (4/~60 commands; Turkish UI text until M7; no SPA CSP; anonymous workspace-creation DoS; `tauri build` not run).
- Next: WAIT for user review and explicit approval of M4.

## Previous checkpoint (M3-CP001)

- Checkpoint ID: M3-CP001 — **IN-PROGRESS CHECKPOINT — NOT A STAGE RELEASE**
- Date: 2026-11-27 (Asia/Phnom_Penh). Stage: M3 (approved).
- Included: web adapter/shims/shell, `vite.web.config.ts`, static serving in server, `docs/M3_NOTES.md`. Files changed: see PROJECT_STATUS "Changed files" (M3-CP001).
- Tests: web cargo 25 pass; Node adapter 6 pass; desktop tsc/vite build pass; web build pass; Chromium E2E 10/10 (after fixing an effect-loop bug found by repetition).
- Gaps: 4/60 commands; UI Turkish until M7; drag-drop untested; no CSP for SPA; tauri build not run.
- Next: finish/verify remaining M3 scope, then READY FOR REVIEW. Do not start M4.

## Previous checkpoint (M3-CP000)

- Checkpoint ID: M3-CP000 — **IN-PROGRESS CHECKPOINT — NOT A STAGE RELEASE** (M3 start snapshot)
- Date: 2026-11-27. Stage: M3 approved by user ("I approve M3" 10:23 +07:00); no M3 code written yet. Contents = M2-CP005 + approval-state edits to status/checklist/handoff/log only.
- Tests: none new. Next: audit `src/api.ts` + components, design HTTP adapter.

## Previous checkpoint (M2-CP005)

- Checkpoint ID: M2-CP005 — **READY FOR REVIEW** (M2 vertical slice). Not approved; M3 NOT started.
- Date: 2026-11-27 (Asia/Phnom_Penh).
- Files changed since CP004: `web/server/static/dev.html`, `web/server/static/dev.js` (new); `web/server/src/app.rs`, `web/server/tests/api.rs`, `web/server/examples/make_sample_rpf.rs`; `docs/M2_SLICE_NOTES.md`, status/checklist/handoff/log.
- Tests: `cargo test --workspace` (web/) 24 passed, 0 failed; Playwright/Chromium flow against the real binary passed; release build OK and smoke-tested; desktop source unchanged vs M2-CP001.
- Known gaps: `docs/M2_SLICE_NOTES.md` (synthetic archives only; memory unmeasured; no Docker image; no editing/saving).
- Next: WAIT for user review and explicit approval of M3.

## Previous checkpoint (M2-CP004)

- Checkpoint ID: M2-CP004 — **IN-PROGRESS CHECKPOINT — NOT A STAGE RELEASE**
- Date: 2026-11-27 (Asia/Phnom_Penh). Stage: M2 (approved).
- Included: HTTP layer + tests + notes. Files changed since CP003: `web/server/src/{app,main,config,store,lib}.rs`, `web/server/{Cargo.toml,tests/api.rs,examples/make_sample_rpf.rs}`, `web/Cargo.lock`, `docs/M2_SLICE_NOTES.md`, status/checklist/handoff/log.
- Tests: `cargo test --workspace` in `web/`: 23 passed, 0 failed. Manual curl run of the real binary: all expected results. Desktop source diff vs CP001: unchanged.
- Gaps: see `docs/M2_SLICE_NOTES.md` (no UI harness, no Docker/release build, synthetic archives only, memory use unmeasured).
- Next: batch 3 (dev harness + docs). Do not start M3.

## Previous checkpoint (M2-CP003)

- Checkpoint ID: M2-CP003 — **IN-PROGRESS CHECKPOINT — NOT A STAGE RELEASE**
- Date: 2026-11-27 (Asia/Phnom_Penh). Stage: M2 (approved). D3 approved by user for `web/` crates.
- Files changed since M2-CP002: `web/.cargo/config.toml` (new), `web/Cargo.lock` (re-resolved), status/checklist/handoff/log.
- Tests: `cargo test -p zyn-doc-engine` 6/6 pass; `cargo test -p tlk-rpf-core` (scratch copy) 16+14 pass. Server crate tests unchanged (8/8 via rustc earlier).
- Gaps: no HTTP layer; release build and Docker untested; frontend/Tauri builds not run.
- Next: batch 2b HTTP layer. Do not start M3.

## Previous checkpoint (M2-CP002)

- Checkpoint ID: M2-CP002 — **IN-PROGRESS CHECKPOINT — NOT A STAGE RELEASE**
- Date: 2026-11-27 (Asia/Phnom_Penh)
- Stage: M2 vertical slice (approved); batch 2a = `web/server` std-only config + workspace store.
- Files changed since M2-CP001: `web/server/Cargo.toml`, `web/server/src/lib.rs`, `web/server/src/config.rs`, `web/server/src/store.rs` (new); `PROJECT_STATUS.md`, `FEATURE_CHECKLIST.md`, `HANDOFF.md`, `CHECKPOINT_LOG.md`.
- Tests: `rustc --edition 2024 --test web/server/src/lib.rs` → 8 passed, 0 failed. `doc-engine/src/paths.rs` via stub → 2 passed. Nothing else compiled (no crate cache; D3 unapproved). SHA-256 is in the delivery message and `.sha256` sidecar.
- Known gaps: no HTTP layer; doc-engine (except paths.rs) uncompiled; no `meta.json`/backups/`current`; `/dev/urandom` randomness (Unix only).
- Next action: user answers D3; then batch 2b. Do not start M3.
- Previous: M2-CP001 preserved (user holds it).

## Previous checkpoint (M2-CP001)

- Checkpoint ID: M2-CP001 — **IN-PROGRESS CHECKPOINT — NOT A STAGE RELEASE**
- Stage: M2 vertical slice (approved 2026-11-26 20:00 +07:00); batch 1 of 3 = `web/` workspace + `zyn-doc-engine` crate (+unit tests written).
- Included: everything from M1-CP001 plus `web/Cargo.toml`, `web/Cargo.lock` (copy of root lock to pin core deps — unverified), `web/doc-engine/**`.
- Tests: **written but NOT compiled/run** (no crate cache, no network). No result exists.
- Next action: batch 2 = `web/server` (config, workspace store, session cookie, upload/download, documents routes, tests); batch 3 = dev harness page + docs.
- Assumptions: D1=A, D2=English DTOs, D5=env keys (user had not answered D1–D6). New deps NOT approved by user; none fetched.

## Previous checkpoint (M1-CP001)

- Checkpoint ID: M1-CP001 — **READY FOR REVIEW** (M1 architecture, design only)
- Date: 2026-11-26 (Asia/Phnom_Penh)
- Stage: M1 — user approved start ("approve M1"); M2 NOT approved
- Included: full project + control docs + `docs/M0_SOURCE_AUDIT.md`, `docs/M1_ARCHITECTURE.md`, `docs/M1_API_CONTRACT.md`.
- Files changed since M0-CP001: `docs/M1_ARCHITECTURE.md` (new), `docs/M1_API_CONTRACT.md` (new), `PROJECT_STATUS.md`, `FEATURE_CHECKLIST.md`, `HANDOFF.md`, `CHECKPOINT_LOG.md`. No code changed.
- Tests/checks: none run (documentation only). Baseline still blocked (no deps).
- Next action: WAIT for user review, answers to D1–D6, and explicit M2 approval.
- Previous: M0-CP001, M0-CP000, FOUNDATION-CP001 preserved.

## Previous checkpoint (M0-CP001)

- Checkpoint ID: M0-CP001 — **READY FOR REVIEW** (M0 source audit)
- Date: 2026-11-26 (Asia/Phnom_Penh)
- Stage: M0 — approved by user 2026-11-25; next stage M1 NOT approved
- Included: full project source + all control documents + new `docs/M0_SOURCE_AUDIT.md`; updated PROJECT_STATUS / FEATURE_CHECKLIST / HANDOFF / this log.
- Files changed since M0-CP000: `docs/M0_SOURCE_AUDIT.md` (new), `PROJECT_STATUS.md`, `FEATURE_CHECKLIST.md`, `HANDOFF.md`, `CHECKPOINT_LOG.md`. No application source changed.
- Tests/checks: `cargo test --offline` failed at dependency resolution (no crate cache); frontend build not run (no node_modules). No test passed or failed. SHA-256 is provided in the delivery message and the `.sha256` sidecar file (a file cannot contain its own hash).
- Known gaps: other `core/src` modules and React components read by grep/signature only; baseline blocked.
- Next action: WAIT for user review/approval of M1.
- Previous checkpoints preserved: M0-CP000, FOUNDATION-CP001.

## Superseded entry (M0-CP000)

- Checkpoint ID: M0-CP000 (starting snapshot, packaged 2026-11-25 23:19+07:00 session start)
- Stage: M0 — NOT STARTED; approval for M0 NOT yet received
- Status: IN-PROGRESS CHECKPOINT — NOT A STAGE RELEASE
- Included: Unmodified foundation source and control documents, plus this log entry. No application code changed.
- Tests: Not run (no audit or build begun).
- Next action: WAIT for the user's explicit approval to begin M0. Do not start the audit until approved.
- Previous checkpoint FOUNDATION-CP001 preserved (original upload ZIP kept unchanged).

## Previous checkpoint

- Checkpoint ID: FOUNDATION-CP001
- Date: 2026-11-25
- Stage: Foundation package / before M0 source audit
- Status: IN-PROGRESS CHECKPOINT — NOT A STAGE RELEASE
- Included: Original Zyn-RPF source plus Claude workflow, approval-gate, and usage-limit delivery documents.
- Application implementation: Not started.
- Tests: Not run; this package adds workflow documentation only.
- Next action: Begin M0 source audit only when the user authorizes it; create and deliver checkpoints during the audit.

## Checkpoint history

| Checkpoint | Stage | Status | Notes |
|---|---|---|---|
| FOUNDATION-CP001 | Foundation | In-progress baseline | Original source plus workflow documentation; no app code changed |
| M5a-CP001 | M5a READY FOR REVIEW | Review package | visual settings editor through web; docs/M5A_NOTES.md |
| M4-CP003 | M4 READY FOR REVIEW | Review package | archive workflows through web; E2E 15 steps; docs/M4_NOTES.md |
| M4-CP002 | M4 in progress | In-progress checkpoint | server routes, versions, backups; 5 route tests |
| M4-CP001 | M4 in progress | In-progress checkpoint | edit engine + 18 tests; no routes yet |
| M3-CP002 | M3 READY FOR REVIEW | Review package | adapter+shell verified; final suite green |
| M3-CP001 | M3 in progress | In-progress checkpoint | web adapter+shell works E2E |
| M3-CP000 | M3 start | Start snapshot | M2-CP005 + approval recorded |
| M2-CP005 | M2 READY FOR REVIEW | Review package | slice complete; 24 tests; browser + release verified |
| M2-CP004 | M2 in progress | In-progress checkpoint | HTTP layer works; 23 tests + curl smoke |
| M2-CP003 | M2 in progress | In-progress checkpoint | deps fetched; doc-engine 6/6, core 16+14 pass; linker fix |
| M2-CP002 | M2 in progress | In-progress checkpoint | web/server std-only config+store, 8 rustc tests pass |
| M2-CP001 | M2 in progress | In-progress checkpoint | web workspace + doc-engine, uncompiled |
| M1-CP001 | M1 READY FOR REVIEW | Review package | Architecture + API contract docs; design only |
| M0-CP001 | M0 READY FOR REVIEW | Review package | Audit document `docs/M0_SOURCE_AUDIT.md`; no app code changed; baseline blocked |
| M0-CP000 | M0 (not started) | In-progress baseline | Starting snapshot of the unmodified source; only CHECKPOINT_LOG.md and HANDOFF.md session note edited |

Add a new entry every time a checkpoint ZIP is created and delivered. Never record a ZIP as delivered unless it was actually attached or made available to the user.
