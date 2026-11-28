## Critical: send checkpoint files before usage limits

Before doing substantial work, read `CHECKPOINT_AND_USAGE_LIMIT_PROTOCOL.md`. The user needs actual downloadable ZIPs before Claude's free usage is exhausted, not just a final ZIP after the stage.

Create an initial snapshot immediately, then send full project checkpoint ZIPs after every meaningful batch (normally 3–5 changed files or one independently testable unit). If a usage-limit warning appears, stop coding and package/send the latest verified ZIP immediately. Update `CHECKPOINT_LOG.md`, `PROJECT_STATUS.md`, and `HANDOFF.md` every time. Checkpoints do not grant approval to start another stage.

# Zyn-RPF Web Conversion — Claude Start Here

> **Read this file first. Then read `CLAUDE_MASTER_PROMPT.md`, `WEB_CONVERSION_BASELINE.md`, `FEATURE_CHECKLIST.md`, and `PROJECT_STATUS.md`.**
>
> This repository is the original Zyn-RPF/Tauri application. The task is to convert this actual project into an English-language web application for GitHub + Railway. It is not a greenfield rewrite.

## Non-negotiable rules

1. **Reuse the source.** Preserve the Rust `core/` crate and its file-format algorithms. Preserve existing React components and workflows wherever practical.
2. **Do not fake completion.** No placeholder buttons, mock data presented as real data, stubbed API responses, or claims that unimplemented features work.
3. **Do not guess.** Inspect the relevant implementation before changing it. If a feature's implementation or behavior is unclear, mark it `NEEDS INVESTIGATION` and document the evidence.
4. **Do not destroy the desktop source.** Keep the original Tauri application buildable unless a specific change is essential and approved. Prefer a separate web backend/adapter and shared core.
5. **English everywhere in the delivered web application.** Translate user-facing text, documentation, errors, labels and newly introduced code identifiers where safe. Do not blindly rename binary-format fields, serialized keys, API contracts or identifiers whose names are part of compatibility.
6. **No dependency installation without explicit user approval.** Do not run `npm install`, `npm ci`, `cargo install`, or equivalent. Inspect manifests and lockfiles. You may run already-available checks/builds/tests if they do not install dependencies or mutate the environment unexpectedly.
7. **Work in small, verifiable milestones.** Update `PROJECT_STATUS.md` and `FEATURE_CHECKLIST.md` whenever work changes. Keep `HANDOFF.md` ready for the next Claude session/account.
8. **Never claim tests passed unless you ran them and saw them pass.** Record exact commands and outcomes.
9. **Do not overwrite user files or original uploaded archives.** Work in this extracted project directory and preserve backups before destructive edits.
10. **Do not silently narrow scope.** If a feature is difficult in a browser, design a web-compatible equivalent and record any unavoidable limitation.

## How to work across Claude accounts / usage limits

At the start of every session:
1. Read `CLAUDE_START_HERE.md`.
2. Read `APPROVAL_GATE.md` — this is mandatory and overrides any automatic-continuation interpretation.
3. Read `HANDOFF.md` (if it contains session notes).
4. Read `PROJECT_STATUS.md` and `FEATURE_CHECKLIST.md`.
5. Check whether the user has explicitly approved the next stage. If not, STOP and wait.
4. Inspect the actual working tree; do not assume the previous handoff is perfectly current.
5. Continue the first unchecked/in-progress task in the stated milestone. Do not restart planning or rewrite completed work.

**Approval gate:** Never proceed to the next milestone without explicit user approval. At the end of every milestone, prepare and verify a full ZIP snapshot, mark the stage READY FOR REVIEW, ask for approval, and STOP. See `APPROVAL_GATE.md`.

Before ending a session or when approaching a usage limit:
1. Update `PROJECT_STATUS.md` with the current milestone, exact completed work, exact remaining work, changed files and test results.
2. Update checkboxes in `FEATURE_CHECKLIST.md`: `[x]` only when implemented and verified; `[~]` for partial; `[ ]` for not done; `[!]` for blocked/needs investigation.
3. Replace `HANDOFF.md` with a concise, self-contained handoff for the next Claude account. Include the exact next action, relevant files, known issues, commands already run, and what must not be repeated.
4. Ensure the project is left in a coherent state. Do not leave half-written files or broken imports.
5. Do not spend the final response on a long progress report. The repository handoff files are the source of truth.

## Status marker meanings

- `[ ]` Not started.
- `[~]` In progress or partially implemented.
- `[x]` Implemented and verified against stated acceptance criteria.
- `[!]` Blocked, unsupported, or requires investigation. Explain why.

A feature is **not** complete merely because a screen exists. It is complete only when its real workflow is connected to the backend/core, errors are handled, and the acceptance criteria are verified.

## Current project facts

- Original project is a Tauri 2 + React + TypeScript + Vite application.
- Rust workspace includes `core/` and `src-tauri/`.
- Existing frontend API wrapper is `src/api.ts` and currently imports Tauri `invoke`.
- Main Tauri command implementation is in `src-tauri/src/lib.rs`.
- Existing core crate is `core/`; inspect its modules before touching format logic.
- `package.json`, `package-lock.json`, `Cargo.toml`, and `Cargo.lock` are present.
- Original README describes the app as a Windows FiveM RPF editor and documents known encrypted archive limitations.
- Do not assume every README-listed feature is complete in the current source. Verify each implementation in code.

## First action

Do not immediately redesign the UI or rewrite the core. First create an evidence-based source inventory and a conversion boundary map:
- Which existing Rust functions can be reused directly from a new web backend?
- Which Tauri commands are filesystem/dialog/window-specific and need web equivalents?
- Which frontend components can remain with only an API adapter change?
- Which APIs currently pass local paths and must be redesigned around uploaded workspace files?
- Which capabilities rely on Windows-only dependencies or OS facilities?

Then implement the smallest working vertical slice: browser upload → server workspace → existing Rust core opens/parses an RPF → frontend displays archive tree → download/export. This validates the architecture before broad conversion.
