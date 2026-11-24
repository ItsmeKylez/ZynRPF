# TECHNICAL DECISIONS (final for planning, Stage 6)
Status values: PROPOSED (needs user approval) / ACCEPTED (user approved) / OPEN.
History: M3 built 2026-11-24 (see M3_REPORT.md). M2 built 2026-11-24 (user replied APPROVED to M2_PROPOSAL.md). M1 reviewed and closed 2026-11-24 (user replied APPROVED; scope chosen by assistant at user's instruction "YOU CHOOSE": close M1 and write an M2 proposal, no code). M1 built 2026-11-24. G0 spikes run 2026-11-24. Stage 3 approved 2026-11-23. Stage 4 approved 2026-11-23. Stage 5 approved 2026-11-23 (user reply `APPROVED`), so D-022..D-029 are ACCEPTED as design, except where a measurement or user answer is still pending (noted in the Status column). Stage 6 was approved 2026-11-23, so D-030..D-035 are ACCEPTED. The six open questions in PRODUCT_SPEC s13 were not answered; spec defaults stand until you say otherwise.

| ID | Decision | Status | Rationale | Stage |
|---|---|---|---|---|
| D-001 | Clean-room, non-GPL implementation; reference studied for format facts only | ACCEPTED as approach; licence MIT (G2 default, accepted by M1 approval) | Avoid GPL-3.0 obligations | 1 |
| D-002 | English only, identifiers/UI/docs/errors; structure ready for i18n later | ACCEPTED by user requirement | | 1 |
| D-003 | Drop desktop-only features (mods folder scan/toggle/install, file lock, AI assistant) in v1 | ACCEPTED | No web equivalent / abuse risk | 2 |
| D-004 | NG/AES encrypted archives: detect and report only in v1 | ACCEPTED; user may still revise | Needs game keys; legal review | 2 |
| D-005 | Parsing must be streaming/range-based, never "whole archive in RAM per tab" | ACCEPTED | Reference peak RAM ~2x archive size | 3 |
| D-006 | Untouched entries copied byte-for-byte on save; only changed entries recompressed | ACCEPTED | Speed and fidelity | 3 |
| D-007 | Validate-before-publish: rebuilt archive must re-parse cleanly before it becomes the new version | ACCEPTED | Reference does this; keep | 3 |
| D-008 | Hard limits: max upload, entries (<=1M), depth (128), nesting (4), decompressed size and ratio, names table 64 KiB | ACCEPTED; numeric values in ARCHITECTURE s8 (see D-029) | Findings S2-S7 | 3 |
| D-009 | Parsers run in isolated worker with timeout and memory cap; crash = failed job | ACCEPTED; mechanism in D-024 | Findings S5 | 3 |
| D-010 | Fuzz/corruption tests in CI for every parser | ACCEPTED; plan in TESTING_STRATEGY s4 | | 3 |
| D-011 | Page-flag/virtual-pointer facts in TECHNICAL_ANALYSIS s2 are the spec; verify against real game files | ACCEPTED; still UNVERIFIED in game | Several UNVERIFIED | 3 |
| D-012 | Backend language and use of `rage-formats` / `intel_tex_2` vs own implementations | OPEN. Path accepted in Stage 5 (Rust, own RPF7/RSC7 code, `rage-formats` only as test oracle, `intel_tex_2` gated by S3); final decision after spikes. G0 (2026-11-24): own RPF7 parser/rebuilder in std-only Rust works (F1/F2 PASS, synthetic); rage-formats and intel_tex_2 build check still BLOCKED (needs Windows PC/network) | Portability unverified | 5 |
| D-013 | Undo as an operation log rather than archive snapshots | ACCEPTED | Audit, cheaper memory | 2-3 |
| D-014 | Backups become workspace versions (not disk paths) | ACCEPTED | Web model | 3 |
| D-015 | Identity: accounts (email+password, argon2id) plus short-lived anonymous sessions | ACCEPTED as design in Stage 5; final choice still depends on your answer to open question 2 | Workspaces need an owner | 4-5 |
| D-016 | Retention: anonymous 24 h idle, accounts 30 days idle, version cap 10, real deletion | ACCEPTED (spec default; values may change after your hosting answer) | Cost and privacy | 4 |
| D-017 | Hex viewer read-only plus explicit logged "Patch bytes" operation | ACCEPTED (spec default; open question 5 unanswered) | Power use without silent edits | 4 |
| D-018 | WCAG 2.1 AA target; desktop-first, phone non-goal in v1 | ACCEPTED | Scope control | 4 |
| D-019 | Terms: users must own rights to uploads; no game assets or keys shipped | ACCEPTED | Legal exposure | 4 |
| D-020 | One writer per workspace at a time; others read-only | ACCEPTED | Avoids merge conflicts | 4 |
| D-021 | Validation is a standalone job with stable check codes, also run inside Save | ACCEPTED; registry in TESTING_STRATEGY s6 (D-031) | Differentiator plus D-007 | 4 |
| D-022 | Hosting topology v1: one `app` service (API + worker processes + SPA) with one volume, plus PostgreSQL; storage behind an interface so S3-compatible storage and a separate worker service can follow | ACCEPTED as design; Railway rules UNVERIFIED (spikes S1, S5) | Fewest moving parts | 5 |
| D-023 | Job queue in PostgreSQL (`FOR UPDATE SKIP LOCKED`), progress via SSE, no Redis in v1 | ACCEPTED | Small queue; DB already required | 5 |
| D-024 | Worker = separate executable, one process per job, unprivileged user, rlimits, no network, path-scoped; supervisor kills process group; output re-validated by API | ACCEPTED as design; feasibility on Railway UNVERIFIED (spike S4) | Implements D-009 | 5 |
| D-025 | API: REST/JSON under `/api/v1`, HTTP Range for byte access, resumable chunked upload, cookie sessions + CSRF, stable error codes with catalogue messages | ACCEPTED | See API_SPEC | 5 |
| D-026 | Per-version SQLite index file for entries; Postgres holds metadata only | ACCEPTED | 1M-entry archives, fast paged tree queries | 5 |
| D-027 | Each version stored as a full archive file; version cap and byte quota bound cost; delta/shared storage deferred | ACCEPTED | Simplicity, any version downloadable | 5 |
| D-028 | Dependency licence gate (MIT/Apache-2.0/BSD/ISC/MPL-2.0 only; `cargo deny` in CI) | ACCEPTED | Enforces D-001 | 5 |
| D-029 | Final v1 numeric limits and quotas as in ARCHITECTURE s8 | ACCEPTED as starting values, UNMEASURED; revisit at gate G1 after S5-S6 and your plan/budget | Resolves PRODUCT_SPEC s9 | 5 |
| D-030 | Fixtures are generated or hand-built synthetic files only; no game assets in the repository or CI; real samples (if supplied) stay outside the repo | ACCEPTED | D-004, D-019 | 6 |
| D-031 | Validation check codes form an append-only registry: never renamed or reused; any Error blocks publish; severity changes need a decision entry | ACCEPTED | Stable contract for UI, API and tests | 6 |
| D-032 | English audit (non-ASCII scan, spell check, catalogue coverage, no literal UI text) is a CI gate and a launch gate; user data is exempt | ACCEPTED | Enforces D-002 | 6 |
| D-033 | Gated progression: spikes need explicit approval (G0), D-012 final at G1, implementation needs separate approval (G2); phases P1-P8 each end with a user review (G3) | ACCEPTED | User control of progress | 6 |
| D-034 | Phased roadmap P1-P8 with risk register R1-R16 maintained in ROADMAP.md and reviewed at each G3 | ACCEPTED | Keeps risks visible | 6 |
| D-035 | Acceptance N (in-game compatibility) is reported honestly as not verified unless real game files and a manual in-game check exist | ACCEPTED | Avoids false completion claims | 6 |
| D-036 | G0 is run as five combined spikes F1-F5 in speed order; results in G0_RESULTS.md | ACCEPTED (user request 2026-11-24) | User priority: speed | G0 |
| D-037 | Own RPF7 index parser/rebuilder is feasible and stays the plan; large-resource marker 0xFFFFFF must be handled; names bounded by 16-bit file name offsets (PARTIAL) | ACCEPTED 2026-11-24 (M1 review) | Spike F1/F2 evidence, synthetic only; real-game verification still pending (D-011, D-035) | G0 |
| D-038 | MVP-first slice for G2 (open, browse, edit, save, download), rest of P1-P8 incremental; phases M1-M4, each needs approval | ACCEPTED 2026-11-24 (user approved M1 with the default package) | User goal: working MVP fast | G0 |
| D-039 | M1 temporary deviations: std-only HTTP layer; in-memory metadata and index cache instead of SQLite/Postgres; parse in an in-process thread instead of the sandboxed worker; polling instead of SSE; Linux-only random source. Each must be removed or re-approved before public launch (sandbox before any public exposure) | ACCEPTED 2026-11-24 as temporary M1 deviations only; each still must be removed or re-approved before public launch | No crate registry offline; keeps M1 small | M1 |
| D-040 | M2 (web UI) scope as written in M2_PROPOSAL.md: browse, upload, search, entry detail, read-only hex view, download; no editing | ACCEPTED 2026-11-24 (user APPROVED); framework React 18 + Vite 5 + TypeScript chosen by assistant per the proposal's stated preference | Smallest UI that exercises the M1 API | M2 |
| D-041 | M2 small backend changes: serve static UI from WEB_DIR (preloaded files only), `/me` returns the CSRF token so a reload keeps the session, default read rate limit 120 to 600 per minute | ACCEPTED 2026-11-24 (user APPROVED after the M2 report listed it) | UI needs them; value unmeasured | M2 |
| D-042 | npm dependency licences (React, Vite, TypeScript, axe-core test-only) are not yet covered by the D-028 gate | OPEN | Add a licence check before launch | M2 |
| D-043 | M3 scope as written in M3_PROPOSAL.md: logged edit operations, undo/redo, save as new validated version, one writer, stored bytes only | ACCEPTED 2026-11-24 (user replied APPROVED; assistant read it as approving M3 as written with defaults, please confirm); BUILT, see M3_REPORT.md | Smallest useful editor | M3 |
| D-044 | M3 additions: write rate default 30 to 120 per minute; blob upload and edit/version routes; 30 s writer lease; 8 MiB body drain on early rejection; browse shows the base version until Save | PROPOSED, needs your confirmation | Needed by the editor; values unmeasured | M3 |
| D-045 | M4 as instructed by the user ("just do the code, I'll deploy it myself"): deployment packaging only (Dockerfile, entrypoint, railway.json, DEPLOY_RAILWAY.md), GLOBAL_QUOTA_BYTES cap (default off), container starts as root only to prepare the volume; nothing deployed by the assistant; D-039 blockers remain | PROPOSED, needs your confirmation | User deploys; protects the volume | M4 |
| D-046 | M5a delivered as a frontend-only slice (lazy tree, summary, shortcuts, Save and download, theme) because of the usage limit; backend items of M5a stay open | PROPOSED, needs your confirmation | Limit on session usage; backend untouched so M1-M4 results stay valid | M5a |
| D-047 | Read-only preview route GET /workspaces/{id}/edit/tree (replays pending ops, lists one folder, 1 to 500 per page); tree shows pending edits, grid still shows the base version | PROPOSED, needs your confirmation | Pending edits were invisible until Save (F-06); additive, no route changed | M5a |

## Decisions still needing your input
1. D-001 licence for new code.
2. D-015 accounts vs anonymous; CAPTCHA; email in v1.
3. D-016/D-029 quota and retention values, Railway plan and budget.
4. D-017 hex "Patch bytes" in v1.
5. Sample archives (affects D-011, D-012, D-035).
6. No volume backups in v1 (RAILWAY_DEPLOYMENT s11).
7. Upload ceiling: PRODUCT_SPEC s9 says 2 GiB, D-029 starts at 1 GiB (hard ceiling 2 GiB) until spikes S5-S6 give measurements. Confirm that is acceptable.

Resolved by re-attached sources: US English spelling is already fixed by PRODUCT_SPEC s10 (part of D-002).
