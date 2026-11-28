## Usage-limit safety

Read `CHECKPOINT_AND_USAGE_LIMIT_PROTOCOL.md` and `CHECKPOINT_LOG.md`. Always deliver a verified checkpoint ZIP after meaningful work batches and immediately when a usage-limit warning appears. Do not wait for stage completion. The latest delivered checkpoint must be named in `CHECKPOINT_LOG.md`; if newer edits exist, package them before ending the session.

# HANDOFF — Read this before continuing

## Approval gate — check before doing anything

Read `APPROVAL_GATE.md`. Current permission to proceed is **NO** unless the user has explicitly approved the stage in the conversation and `PROJECT_STATUS.md` has been updated accordingly.

Do not continue into a new milestone automatically. Deliver one milestone, prepare a verified ZIP, ask for approval, and stop.

## Current state (updated 2026-11-26, M1)
- M0 approved and delivered (M0-CP001). **M1 — Architecture: READY FOR REVIEW** (user approved starting M1: "approve M1", 2026-11-26 19:52 +07:00). **M2 is NOT approved. Permission to begin next stage: NO.**
- M1 deliverables (design only, no code): `docs/M1_ARCHITECTURE.md` (components, workspace/session model, security, memory/jobs, Railway, test plan, M2 slice definition, decisions D1–D6) and `docs/M1_API_CONTRACT.md` (proposed REST routes covering all 60 Tauri commands: 57 mapped, 3 removed with reasons).
- Still blocked from M0: no baseline test/build (no crate cache / node_modules); no sample `.rpf` files. New crates (axum, tower-http, zip, uuid/rand, tracing) are NOT in `Cargo.lock`; adding them needs user approval + network.

## Latest session (2026-11-27, M2-CP002)
- Verified uploaded M2-CP001 by SHA-256, extracted, re-read all control docs. M2 is approved/in progress; M3 NOT approved.
- Wrote `web/server` (std-only: `config.rs`, `store.rs`) — 8 tests pass with plain `rustc` (cargo has no crate cache). Verified `doc-engine/paths.rs` (2 tests) via stub.
- Did NOT add axum/tokio/tower-http/zip: D3 still unanswered. `doc-engine` (except paths.rs) and the `web/` workspace have never been compiled with cargo.
- Stale dirs `extract/` and `extracted/` may exist beside the project in the authoring sandbox; ignore, not part of the project.

## STATE (M5a-CP001, 2026-11-28): M5a (visual settings) READY FOR REVIEW — next M5 family NOT approved
- M4 accepted and M5 approved by the user's answer "M4 accepted — start M5" (2026-11-28 21:37 +07:00). M5 is delivered family by family; M5a = visual settings (see `docs/M5A_NOTES.md`). **STOP and wait for explicit approval of the next family** (suggested: M5b texture editor: `ytd_listesi`, `doku_*`, `ytd_islem`, previews). Do not treat a bare "APPROVED"/"Continue" as approval of a new family; ask explicitly (the user has accepted this each time).
- Pattern to reuse for each family: engine module in `web/doc-engine` calling the core (no re-implementation) → routes in `web/server/src/app.rs` → wire type (`src/web/wire.ts`) + mapper (`adapt.ts`, Node test) → http client + `transport.ts` case → Chromium E2E in `web-tests/` using the polling `until` helper (never `waitForFunction(async…)`).
- Verify before any change: `cd web && cargo test --workspace && cargo test -p tlk-rpf-core`; `node --test web-tests/adapt.test.mjs`; `npx tsc --noEmit`; `npm run build:web`; E2E: `NODE_PATH=$(npm root -g) node web-tests/e2e-m4.cjs "$PWD" <outdir>` and `e2e-m5a-settings.cjs` (prereqs in their headers).
- Open items for the user: real `.rpf` / settings samples (none), Railway RAM/disk limits, D6 licence.

## (Earlier) STATE (M4-CP003, 2026-11-28): M4 READY FOR REVIEW — M5 NOT APPROVED
- M4 is complete per its exit criterion (see `docs/M4_NOTES.md`: design, routes + deviations, mapped commands, verification, limitations). **STOP and wait for the user's explicit approval of M5.** If the user asks for M4 changes, do only those.
- Do not treat a bare "APPROVED"/"Continue" as M5 approval; ask an explicit question (the user has accepted this each time).
- Open items for the user: real `.rpf` samples (still none), Railway RAM/disk limits, D6 licence.
- Verify before any change: `cd web && cargo test --workspace && cargo test -p tlk-rpf-core`; `node --test web-tests/adapt.test.mjs`; `npx tsc --noEmit`; `npm run build:web`. E2E: `web-tests/e2e-m4.cjs` (see its header for prerequisites); its 15 steps are listed in `docs/M4_NOTES.md`.

## (Earlier) STATE (M4-CP002): server routes + store versions/backups done and tested (`web/server/tests/m4.rs`); next = frontend mapping (see PROJECT_STATUS next action). Route deviations from M1 contract: import/replace take multipart directly (`?dir=`, `?path=`); backups are per file (`/files/{id}/backups`, `/documents/{id}/restore`).

## (Earlier) STATE (M4-CP001, 2026-11-28): M4 APPROVED, IN PROGRESS — batch 1 (engine) done, batch 2 next
- Engine ops + tests are in `web/doc-engine/{src/edit.rs,entry.rs,export.rs,tests/edit.rs}`. `cargo test --workspace` in `web/`: 8+18+8+11 pass. Next: batch 2 (see PROJECT_STATUS next action).

## (Earlier) STATE (M4-CP000)
- M3 accepted ("APPROVED"); user then explicitly confirmed "Yes, start M4". M5 NOT approved. Existing dependency approvals only (web Rust crates, `npm ci`); anything new needs a question first.

## (Earlier) STATE (2026-11-28): M3 APPROVED; M4 START NOT YET CONFIRMED
- User replied "APPROVED" to the M3 review. That approves M3. It did not name M4, so M4 is NOT started; confirmation was requested. Docs-only edit; no code changed.

## (Earlier) STATE (M3-CP002, 2026-11-27): M3 READY FOR REVIEW
- M3 complete per the milestone table ("Typed HTTP adapter and web shell work"). Final verification re-run passed. **M4 is NOT approved — STOP and wait for the user.** If the user asks for M3 changes, do only those.
- Open items for the user: real `.rpf` samples (still none), Railway RAM/disk limits, D6 licence. M4 will need a decision on how web "save" should work (download vs server-side current version) — see `docs/M1_ARCHITECTURE.md`.

## (Earlier) STATE (M3-CP001, 2026-11-27): M3 IN PROGRESS
- Done: adapter + web shell + static serving; verified in Chromium. `npm ci` approved/run. Notes in `docs/M3_NOTES.md`.
- Remaining M3 candidates (only if in scope): test drag-drop upload in a browser; decide with user whether M3 is "done" given 4/60 commands (the rest are M4+ per the milestone table); then mark READY FOR REVIEW. Do NOT start M4.

## (Earlier) STATE (M3-CP000, 2026-11-27): M3 APPROVED, IN PROGRESS
- User approved M3 ("I approve M3", 2026-11-27 10:23 +07:00). M4 NOT approved. `npm ci` NOT approved — ask before installing. Work only on M3; stop at READY FOR REVIEW.

## (Previous) STATE (CP005, 2026-11-27): M2 READY FOR REVIEW
- M2 vertical slice complete per `M1_ARCHITECTURE.md` §10; dev harness page added; release build verified. 24 cargo tests pass; Chromium run passes. **M3 is NOT approved — STOP and wait for the user.** If the user asks for M2 changes, do only those.
- Open items for the user: real `.rpf` samples (none supplied), Railway RAM/disk (memory use of open-in-RAM design unmeasured), D6 licence.

## Update CP004 (2026-11-27)
- HTTP layer done: `web/server/src/{app,main}.rs`; route tests `web/server/tests/api.rs`; notes in `docs/M2_SLICE_NOTES.md` (routes, deviations, limits). 23 web tests pass; real binary smoke-tested with curl.
- Remaining M2 (batch 3): minimal dev harness page + static serving; update docs; only then mark M2 READY FOR REVIEW and ask approval. Do NOT start M3 (frontend migration) — not approved.
- Do not repeat: dependency fetch, linker fix, core baseline (all done; see log).

## Update CP003 (2026-11-27)
- User APPROVED D3 (Rust crates for `web/` only; no npm). D4/D6 defaults kept. Deps fetched; `cargo test -p zyn-doc-engine` 6/6 pass; core baseline 16+14 pass (scratch copy).
- Linux link needs `-lstdc++` for intel_tex_2 (ISPC prebuilt objects): `web/.cargo/config.toml`. Railway Dockerfile must have libstdc++ (g++ runtime). No ISPC toolchain needed.
- Next: batch 2b = axum HTTP layer in `web/server` on top of `store.rs` + `doc-engine`.

## Pending user decisions
D1 doc-layer location (rec. A: new Tauri-free `web/doc-engine`, desktop untouched) · D2 English DTOs on the wire (rec. yes) · D3 approve new dependencies + fetching for M2 · D4 limits / Railway plan (RAM, disk) · D5 AI keys (rec. operator env keys, no BYO) · D6 licence stance for web code (upstream GPL-3.0-only).

## Exact next task
M2 (approved). Ask/await D3. If approved + network: fetch deps, `cargo test` in `web/` (fix doc-engine compile errors first), then batch 2b = HTTP layer in `web/server` (session cookie, upload, documents open/tree, download, /healthz) on top of `store.rs`; batch 3 = dev harness page + docs. If D3 is not approved: only dependency-free work remains; do not invent more. Do NOT start M3.

## Rules for next session
- Do not restart the project or rewrite the core.
- Do not install dependencies without explicit user approval.
- Do not start M1 without explicit approval.
- If this handoff conflicts with the working tree, inspect the tree; actual code wins.

## Session record
- 2026-11-25 (Claude Web): read control docs, M0 approval was NO, delivered start snapshot M0-CP000, stopped.
- 2026-11-25/26 (Claude Web): user approved M0. Audited `lib.rs` (full), `kasa.rs`, `api.ts`, `modlar.rs` (FS parts), manifests, CI head. Wrote `docs/M0_SOURCE_AUDIT.md`. Baseline blocked (no deps). Packaged M0-CP001 READY FOR REVIEW.
- 2026-11-26 (Claude Web): user approved M1. Wrote M1 architecture + API contract docs (design only). Packaged M1-CP001 (see CHECKPOINT_LOG).
