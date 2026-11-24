# PROJECT STATUS
Last updated: 2026-11-24 (M5a first slice built; awaiting review). Planning closed. G0 spikes done. **Gate G2 approved for phase M1 only; M1 (backend core) is built and tested (see M1_REPORT.md). Phases M2, M3, M4 each need separate explicit approval; nothing further is authorised.**

## Project
New browser-based GTA V / FiveM RPF editor (Railway). Reference: Zyn-RPF (= tlk-rpf, Tauri desktop, GPL-3.0). Clean-room, non-GPL, English only, real backend processing. No coding until planning is approved and implementation is separately approved.

## Workflow rule
One stage at a time. Stop after each stage with a report. Only the user's personal reply `APPROVED` starts the next stage. Questions/acknowledgements are not approval. Change requests revise the current stage.

## Stage checklist
| Stage | Deliverable | State |
|---|---|---|
| 1 Repository discovery | REFERENCE_OVERVIEW.md, PROJECT_STATUS.md, NEXT_SESSION_PROMPT.md | DONE (earlier account); file not in later uploads |
| 2 Feature inventory | FEATURE_MATRIX.md | DONE (earlier account); file not in later uploads |
| 3 Technical analysis | TECHNICAL_ANALYSIS.md, TECHNICAL_DECISIONS.md | DONE - APPROVED 2026-11-23 |
| 4 Product spec | PRODUCT_SPEC.md | DONE - APPROVED 2026-11-23 |
| 5 Architecture | ARCHITECTURE, TECH_STACK, API_SPEC, SECURITY, RAILWAY_DEPLOYMENT | DONE - APPROVED 2026-11-23 |
| 6 Roadmap | ROADMAP, TESTING_STRATEGY, ENGLISH_AUDIT, TECHNICAL_DECISIONS (final), PROJECT_STATUS | DONE - APPROVED 2026-11-23 |
| Build spikes (fast G0: F1-F5) | Throwaway code and measurements | RAN 2026-11-24 on Linux/synthetic: F1,F2,F3,F5 PASS; F4 and oracle-crate check BLOCKED. Awaiting user review |
| Implementation M1 | Backend core (rpf-core + API server) | DONE and REVIEW CLOSED 2026-11-24 (user APPROVED; D-037, D-039, errata 1-6 accepted) |
| Implementation M2 | Web UI | DONE 2026-11-24, awaiting user review (M2_REPORT.md) |
| Implementation M3 | Edit and save | BUILT 2026-11-24, awaiting user review (M3_REPORT.md, D-043, D-044) |
| Implementation M4 | Railway deploy | BLOCKED until user approves |

## Stage 1-5 summary (carried over)
- Reference: Tauri 2 desktop, Rust core ~3.6k LOC + React UI, 60 commands, no server. Hex view only 1 KiB; no identification, structure view, standalone validation, job queue or generic diff (our differentiators). Desktop-only features dropped (D-003).
- Stage 3: RPF7 table, entry kinds, parse/rebuild, RSC7 flags/pointers, texture layout documented. Security gaps for a public service: size-from-header allocations, no decompression limits, minimal name sanitising. Format facts not validated against real game files.
- Stage 4: PRODUCT_SPEC with acceptance A-N, derived without FEATURE_MATRIX.
- Stage 5: Rust API + per-job sandboxed worker + SPA on one Railway service with a volume, plus PostgreSQL; per-version SQLite index; Range reads; jobs in Postgres with SSE; limits in ARCHITECTURE s8; accounts + anonymous sessions; D-022..D-029 accepted as design.

## Stage 6 results (this session)
Files: ROADMAP.md, TESTING_STRATEGY.md, ENGLISH_AUDIT.md, TECHNICAL_DECISIONS.md (final), PROJECT_STATUS.md, NEXT_SESSION_PROMPT.md.
- ROADMAP: gates G0-G4; spike run order S1-S7 with dependencies; phases P1-P8 with exit criteria; risk register R1-R16; launch checklist.
- TESTING_STRATEGY: test layers, generated-only fixtures, hostile fixtures H01-H23 with expected codes, fuzz plan, acceptance A-N mapping, 43-code validation registry, authorisation/sandbox tests, CI gates.
- ENGLISH_AUDIT: scope, scan method, checklist, release rule.
- New decisions D-030..D-035 PROPOSED. D-022..D-029 now ACCEPTED (D-012 still OPEN pending spikes).

## Discovered this stage
- TECH_STACK calls the spikes "throwaway code"; because you require explicit approval before any coding, spikes are gated behind G0.
- Reference file names suggest non-English (apparently Turkish) module names (PARTIAL); nothing from the reference is reused.
- PRODUCT_SPEC.md and TECHNICAL_ANALYSIS.md were re-attached afterwards and cross-checked (see Errata below).

## Open items
- Licence for new code (D-001); accounts vs anonymous (D-015); retention/quota values and Railway plan/budget; sample archives; hex Patch bytes (D-017); FEATURE_MATRIX reconciliation; email in v1 and CAPTCHA; no volume backups in v1.
- Still missing: FEATURE_MATRIX.md and REFERENCE_OVERVIEW.md (scope reconciliation, open question 6).
- D-012 final needs spikes (Rust machine, network, Railway account, ideally samples).

## Errata after re-attaching PRODUCT_SPEC and TECHNICAL_ANALYSIS
Found by reading both documents (VERIFIED against their text). Applied as corrections to Stage 6 and Stage 5 files; they do not change any decision. **Awaiting your confirmation of these corrections.**
1. TESTING_STRATEGY acceptance table had wrong themes for B, C, D, E (actual: B round-trip, C edit, D large resource, E trimmed padding). Rewritten from PRODUCT_SPEC s12. ROADMAP exit criteria and API_SPEC s4 mapping fixed to match.
2. S-numbers confirmed. Fixed: names-table limit is S6 (not S2); S9 added to name tests; S7 and S8 noted. SECURITY header updated.
3. Registry gaps found against PRODUCT_SPEC s6: six codes added (now 43), including `NESTED_PADDING_TRIMMED`.
4. Fixture gaps added: large resource (0xFFFFFF marker), trimmed padding and truncated nested payload, all-zero file start, OPEN/CFXP tags, 300-deep tree, joaat and mip test vectors.
5. US English is already fixed by PRODUCT_SPEC s10; removed as an open question. ENGLISH_AUDIT checklist now includes the s10 copy rules. The reference identifiers are Turkish per TECHNICAL_ANALYSIS (no longer PARTIAL).
6. Upload ceiling differs: PRODUCT_SPEC 2 GiB vs ARCHITECTURE 1 GiB default (hard ceiling 2 GiB). Already allowed by D-029; now listed for your confirmation.

## Verification log
- Stages 1-2: earlier account; not re-verified.
- Stage 3: earlier log (rsc.rs, rpf.rs, ytd.rs, doku.rs, lib.rs, modlar.rs, Cargo.lock read).
- Stage 4: documents only.
- Stage 5: documents only; all Railway, crate, sandbox, performance and capacity claims UNVERIFIED.
- After re-attachment: PRODUCT_SPEC and TECHNICAL_ANALYSIS read fully and compared to Stage 5-6 files.
- Stage 6: documents only. The reference zip was not opened. Nothing was built or run. Check codes, fixture list, risk ratings and roadmap sizes are SUGGESTED, not measured.

## G0 results (2026-11-24) - details in G0_RESULTS.md
- User approved G0 and asked for a speed-optimised plan (F1-F5 combining S1-S7). Code in spikes_g0.zip; std-only because the container has no network.
- F1 PASS (synthetic), F2 PASS, F3 PASS (loopback), F5 PASS. F4 BLOCKED (no Railway account); oracle crates BLOCKED (no network). Not run on the user's Windows PC yet (run_all.ps1 provided).
- Findings: u16 file name offsets bound unique names (PARTIAL); dir/file marker ambiguity (PARTIAL); large-resource marker needs implementation handling.
- Real-game compatibility remains UNVERIFIED (acceptance N). Compression limits, RSC7, real YTD untested.
- Awaiting: user review of G0, optional Windows run and F4 run, then a decision on gate G2.

## M1 results (2026-11-24) - details in M1_REPORT.md
- Approved by user with the default package (MIT, anonymous sessions, no Postgres, 1 GiB upload, 24 h retention, no backups, no hex patching).
- Built: rpf-core (parser, limits, English error catalogue) and server (sessions, CSRF, upload with resume, parse jobs, browse, search, Range reads, download). Code: rpf-editor.zip.
- VERIFIED: rpf-core tests (22 hostile fixtures, 100k fuzz inputs, 0 panics); 70 end-to-end API checks pass; 1M-entry archive through the API: parse 97 ms, server peak 42 MiB.
- Deviations (D-039, need confirmation): std-only HTTP, in-memory metadata and index, in-process parse (no sandbox), no SSE, Linux-only (/dev/urandom).
- Not tested: real game files, compressed entries, TLS/proxy, Railway, load.
- Awaiting: your review of M1, then an explicit decision on phase M2 (web UI).

## M1 review closure (2026-11-24)
- User replied `APPROVED` after the M1 review list. Scope was ambiguous; asked, the user answered "YOU CHOOSE". Assistant chose the narrowest reading: close M1 and write an M2 proposal only.
- Accepted: D-037, D-039 (temporary deviations), errata 1-6. Open questions unanswered; defaults stand.
- Nothing built or run this session. rpf-editor.zip contents were listed only; not compiled or tested here.
- New: M2_PROPOSAL.md and D-040 (PROPOSED). M2 implementation needs a separate explicit `APPROVED` that names M2.

## M2 results (2026-11-24) - details in M2_REPORT.md
- User replied `APPROVED` to M2_PROPOSAL.md. Built: React + Vite + TypeScript UI in `web/` and static serving in the server. Code: rpf-editor-m2.zip.
- VERIFIED in this container (Chromium): 41 browser checks pass, including resume after failure, hex bytes equal to the file, downloads byte-equal, isolation, delete data, and a 1,000,000-entry archive (End key renders in 333 ms, 22 DOM rows). axe-core found no WCAG A/AA violations in four states (one real issue fixed). English audit passes. M1 regression: 70 API checks and rpf-core tests pass.
- NOT verified: real game archives, other browsers, manual screen-reader review, Docker build, TLS/proxy, Railway, load, npm licence gate.
- New: D-041 (small backend changes, PROPOSED), D-042 (npm licence gate, OPEN).
- Awaiting: your review of M2, then an explicit decision on phase M3 (edit and save).

## After M2 report (2026-11-24)
- User replied `APPROVED` to the M2 report, which offered M3 but had no M3 scope. Assistant took it as accepting M2 and D-041 (D-042 stays OPEN), and wrote M3_PROPOSAL.md only. M3 coding needs a further `APPROVED`.

## Recovery check (2026-11-24, new account)
- Claim from the user: a previous session had started M3 backend code. Inspected rpf-editor-m2.zip: it contains M1+M2 only. No M3 code exists (no rebuild, operation log, undo/redo, save, validate or writer-lock code; no TODO or todo!() markers). The status files were already accurate: M3 is a proposal (D-043), not started.
- Not found: spikes_g0.zip (holds the F2 RPF7 rebuilder spike). Needed as reference for the M3 save path unless re-derived.
- VERIFIED in this container: `cargo test --offline` passes (4 rpf-core tests); `cargo build --release --offline` succeeds with 3 dead-code warnings in `server`; M1 e2e regression `scripts/e2e.mjs` passes 70/70 with fixtures `gen good.rpf 5 6291456` and `gen big.rpf 30000 1000` (the script's "50k entries" fixture exceeds the 20 MiB quota it sets, so use 30000; README and script header should be reconciled).
- NOT run: web UI browser tests (no node_modules, no network for npm), fuzz beyond the cargo tests.
- User replied `APPROVED` to the recovery plan. Taken as covering the recovery plan only (status update, baseline), not M3 coding, because plan item 2 states M3 needs a separate APPROVED for M3_PROPOSAL.md.
- M3 remains BLOCKED until the user approves M3 explicitly and answers the three questions in M3_PROPOSAL.md.

## M3 results (2026-11-24) - details in M3_REPORT.md
- No M3 code existed at recovery time. User replied APPROVED three times; the assistant treated the last as approval of M3 as written, with defaults (please confirm).
- Built: rpf-core rebuild + validate (16 new append-only codes), server edit API (op log, undo/redo, writer lease, content upload, dry run, save job, versions cap/delete, standalone validate), web EditPanel.
- VERIFIED: cargo tests (11), M3 API e2e 78/78, M1 e2e 70/70, M3 browser test 23/23 (Chromium), English audit clean.
- NOT verified: real game files (acceptance N), UI tsc/vite build (no network; bundled with local esbuild and React 19), axe scan, M2 browser suite, other browsers, Docker/Railway, large-scale fuzz.
- New: D-044 (PROPOSED). D-042 still OPEN. M4 not started; needs explicit approval.

## M4 packaging (2026-11-24) - details in M4_REPORT.md and DEPLOY_RAILWAY.md
- User said "Just do the code, I'll deploy it myself": treated as approval of M4 as deployment packaging only. Nothing deployed.
- Added Dockerfile (reworked), docker-entrypoint.sh, railway.json, .dockerignore, DEPLOY_RAILWAY.md, GLOBAL_QUOTA_BYTES, safer startup cleanup, scripts/e2e-m4.mjs (6/6). Regressions: M3 e2e 78/78, M1 e2e 70/70. Entrypoint tested without Docker (found and fixed 2 bugs).
- NOT verified: Docker image build (never built), Railway, load, full save time on 1,000,000 entries. D-039 blockers remain for public launch. D-045 PROPOSED.

## Recovery and M5a first slice (2026-11-24, third account)
- Position check: the previous prompt said "Stage 3"; the documents show Stages 1-6 DONE and APPROVED, M1-M4 built, M5 requirements written. Stage 3 was already approved 2026-11-23. The user then replied `APPROVED` to: fix the stale NEXT_SESSION_PROMPT, and make M5a the next stage.
- Built (frontend only, backend unchanged): lazy file tree, Summary panel, keyboard shortcuts, Save and download, theme switch. Details in M5A_REPORT.md.
- VERIFIED: ui-test-m5a 16/16, ui-test-m3 23/23, English audit, cargo test (11). NOT run: tsc/vite build, axe, M2 suite, e2e API suites, real game files, Docker/Railway.
- Open in M5a: F-01 tabs, F-05, F-06, F-08, F-15, Expand all, Export all, header badges. D-046 PROPOSED.
- The user attached Zyn.rpf (RPF7, 144 entries), core.ypt (RSC7) and bloodfx.dat; identified by header only, not parsed.

## M5a part 2 (2026-11-24)
- User replied `APPROVED` to the part-1 report (which offered "finish M5a" or "start M5b"). Read as finish M5a. Done: F-06 live preview of pending edits in the tree (new route GET /workspaces/{id}/edit/tree, Tree::list_folder). See M5A_REPORT.md part 2.
- VERIFIED: ui-test-m5a 20/20, ui-test-m3 23/23, e2e 70/70 and e2e-m3 78/78 (with the settings in their headers), cargo test 11, English audit. NOT run: tsc/vite build, axe, M2 suite, real files, Docker/Railway.
- Still open in M5a: F-01, F-05, F-08, F-15, Expand all, Export all, header badges. D-046 and D-047 PROPOSED.

