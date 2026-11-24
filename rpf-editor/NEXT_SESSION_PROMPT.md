# NEXT SESSION PROMPT (self-contained)

Project: design (then build) a NEW clean-room, non-GPL, English-only, browser-based GTA V / FiveM RPF editor with backend processing, deployed on Railway. Reference app Zyn-RPF (tlk-rpf, Tauri + Rust + React, GPL-3.0) is studied, not ported or cloned.

Rules: ONE stage at a time; after each stage stop, report (done, discovered, files, issues, next steps), update PROJECT_STATUS.md, NEXT_SESSION_PROMPT.md, TECHNICAL_DECISIONS.md; wait for the user's personal reply `APPROVED`. Questions or acknowledgements are not approval. Change requests revise the current stage. No code of any kind (including spikes) before explicit approval. Label findings VERIFIED / PARTIAL / UNVERIFIED / SUGGESTED; never claim completion without verification; keep usage low.

Read first (attach PRODUCT_SPEC.md and TECHNICAL_ANALYSIS.md too): PROJECT_STATUS.md, NEXT_SESSION_PROMPT.md, ROADMAP.md, TESTING_STRATEGY.md, TECHNICAL_DECISIONS.md, then ARCHITECTURE.md, TECH_STACK.md, API_SPEC.md, SECURITY.md, RAILWAY_DEPLOYMENT.md, ENGLISH_AUDIT.md. Do not redo Stages 1-6.

CURRENT POSITION: Planning closed (Stages 1-6). G0 spikes done (G0_RESULTS.md). Gate G2 approved for phase M1 only. M1 (backend core: rpf-core + API server) is built, verified and its review is CLOSED (user APPROVED 2026-11-24; D-037, D-039, errata accepted). M2 (web UI) is built and verified (M2_REPORT.md, rpf-editor-m2.zip): 41 browser checks, axe scan clean, English audit clean, M1 regression passes. M1 (M1_REPORT.md): 70 end-to-end checks and rpf-core tests pass on Linux with synthetic archives. Defaults in force: MIT licence, anonymous sessions only, no Postgres, 1 GiB upload ceiling, 24 h retention, no volume backups, no hex patching.
- Do NOT start M3 (edit and save) or M4 (Railway deploy) until the user explicitly approves that phase. A bare APPROVED after unrelated items is ambiguous, so ask what it covers.
- M2 accepted (D-041 accepted, D-042 open). M3_PROPOSAL.md (D-043) is written and awaits approval.
- Next: wait for the user's reply on M3_PROPOSAL.md. Only an `APPROVED` given after seeing it starts M3 coding. Revise the proposal if the user asks for changes.
- Reminder: the server is Linux-only (/dev/urandom); the user's Windows PC needs WSL or Docker to run it. Spike F4 (Railway) is still unrun.

Open questions for user: licence for new code (D-001); accounts vs anonymous (D-015); retention/quota values and Railway plan/budget; sample archives; hex "Patch bytes" in v1 (D-017); FEATURE_MATRIX reconciliation; email in v1 and CAPTCHA; no volume backups in v1; upload ceiling 1 GiB vs 2 GiB.
PRODUCT_SPEC.md and TECHNICAL_ANALYSIS.md were reconciled (errata in PROJECT_STATUS, awaiting user confirmation). Still missing: FEATURE_MATRIX.md and REFERENCE_OVERVIEW.md. Reference source only needed if a fact must be re-checked: user re-attaches Zyn-RPF-main.zip.

RECOVERY UPDATE (2026-11-24): The zip rpf-editor-m2.zip has no M3 code; M3 was never started. Baseline verified: cargo tests pass, e2e 70/70 (fixtures: gen good.rpf 5 6291456; gen big.rpf 30000 1000). Web UI tests not re-run. spikes_g0.zip (RPF7 rebuilder spike) is not attached; ask the user for it or re-derive.
- Next: wait for the user's explicit APPROVED naming M3 (and answers to the three questions in M3_PROPOSAL.md). Planned order once approved: rpf-core rebuilder and validator with round-trip and hostile tests, then operation log and versions in the server, then writer lock, then UI.

M3 UPDATE (2026-11-24): M3 (edit and save) is BUILT and tested; see M3_REPORT.md and rpf-editor-m3.zip. Verified: cargo tests, e2e-m3 78/78, e2e 70/70, ui-test-m3 23/23, English audit. NOT verified: real game files (acceptance N), UI tsc/vite build and axe scan (run `npm ci && npm run build` and the M2 suite on a machine with network), Railway.
- Next: wait for the user's review of M3_REPORT.md (confirm D-044, the reading of APPROVED as M3 approval, and the known limits). Do NOT start M4 until the user explicitly approves it.

M4 UPDATE (2026-11-24): deployment packaging is written (see M4_REPORT.md, DEPLOY_RAILWAY.md); the user deploys it themselves. The Docker image was never built by the assistant. Next: wait for the user's report from their first build/deploy (build log errors, Railway behaviour) and fix what they report. Remaining launch blockers are in DEPLOY_RAILWAY.md (D-039 sandbox, persistence, accounts, D-042, real-file testing).
