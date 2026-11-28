## CRITICAL: Deliver files BEFORE free usage runs out

Read and obey `CHECKPOINT_AND_USAGE_LIMIT_PROTOCOL.md`. This is a top-priority delivery requirement.

The user has repeatedly lost near-complete work because a free usage limit interrupted Claude before a ZIP was delivered. Do not repeat that workflow.

- Create and send an initial downloadable ZIP snapshot before substantial implementation.
- During the stage, save and deliver a full project checkpoint ZIP after every meaningful, self-contained batch (normally 3–5 changed files or one independently testable unit).
- Before a long/risky operation, deliver a checkpoint ZIP.
- If any usage-limit warning appears or you otherwise know the session may end, stop implementation immediately, save, update handoff/status/log, package and verify the latest full ZIP, and send it to the user.
- Never wait for the stage to be 100% complete before delivering a file.
- Label interim ZIPs clearly as `IN-PROGRESS CHECKPOINT — NOT A STAGE RELEASE`.
- Keep the approval gate intact: checkpoint delivery does not authorize the next stage.
- Never claim the exact remaining usage is known unless the interface explicitly shows it. The solution is frequent early delivery, not guessing the cutoff.
- Do not finish a response with only a progress report when a recoverable ZIP can be created and attached.

The user wants the actual file, not merely a promise that the file is being worked on.

# MASTER PROMPT — ZYN-RPF WEB APPLICATION CONVERSION

You are the lead engineer converting the supplied Zyn-RPF source into a production-oriented English-language web application for GitHub and Railway.

## Mission

Convert the **actual existing project** into a web application while preserving its existing functionality and file-format compatibility. This is a source-code conversion, not a rebuild or a visual mockup exercise.

The source repository is authoritative. Read and understand it before making changes. Reuse existing implementations instead of recreating them.

## Mandatory user approval — highest priority

Read and obey `APPROVAL_GATE.md`. It overrides any instruction to continue development automatically.

**Work one milestone at a time. After completing each milestone, test it, update all status documents, create and verify a full project ZIP, provide a concise change/test report, ask the user for explicit approval, and STOP. Do not begin the next milestone until the user explicitly approves it.**

A request for changes means revise the current milestone only. A request for a ZIP or a progress report is not approval. When switching Claude accounts, inspect the approval state in `PROJECT_STATUS.md` and wait if the next stage is not approved.

The first milestone M0 is source audit only. Do not begin implementation until M0 has been reviewed and the user approves moving forward.

## Required reading and living documents

Before coding, read:
- `CLAUDE_START_HERE.md`
- `APPROVAL_GATE.md`
- `WEB_CONVERSION_BASELINE.md`
- `FEATURE_CHECKLIST.md`
- `PROJECT_STATUS.md`
- `HANDOFF.md`

Keep `PROJECT_STATUS.md`, `FEATURE_CHECKLIST.md`, and `HANDOFF.md` current throughout development. These files allow another Claude account to continue without relying on chat history.

## Existing source architecture

The supplied project contains:
- `core/`: Rust library with archive/resource processing modules.
- `src-tauri/`: Tauri desktop host and command implementations.
- `src/`: React + TypeScript UI and API wrapper.
- `package.json` / `package-lock.json`: frontend tooling and dependencies.
- `Cargo.toml` / `Cargo.lock`: Rust workspace/dependencies.

Important existing files:
- `src/api.ts`: current frontend API contract; uses `@tauri-apps/api/core` `invoke`.
- `src/App.tsx`: application shell.
- `src-tauri/src/lib.rs`: large Tauri command surface and desktop-specific orchestration.
- `core/src/*.rs`: existing format logic. Preserve it.
- `src-tauri/src/kasa.rs`: existing credential storage integration; do not expose secrets to browsers.

Do not treat this inventory as a substitute for inspecting the source.

## Architecture target

Create a web architecture with a React + TypeScript frontend and a Rust HTTP backend that reuses the existing Rust core.

Recommended separation:
- `core/`: shared processing library; retain existing algorithms and tests.
- `web-backend/` (or another clearly named new Rust crate): HTTP routes, upload/workspace lifecycle, request validation, authentication/session boundary if applicable, and calls into `core/`.
- Existing `src/`: reuse components; replace Tauri-only API calls with a typed HTTP client/adapter.
- `docs/`: architecture, API, deployment, and handoff documentation.

Choose a suitable Rust web framework (Axum is acceptable) only after checking current dependencies and compatibility. Do not introduce unnecessary libraries.

### Core boundary
The web backend must call the existing Rust core. Do not port archive parsing, compression, texture processing, RSC/resource writing, YPT logic, model extraction, or other binary-format algorithms into JavaScript.

### Browser boundary
A normal website cannot read arbitrary local paths or silently install files into the user's FiveM directory. Replace these desktop workflows with:
- File upload and drag/drop.
- Folder upload where browser support permits.
- A server-side per-session workspace.
- Safe listing and editing within that workspace.
- Export/download of individual files or ZIP bundles.
- Explicit user action for every destructive save/export.

Do not claim the website can directly modify a user's local FiveM mods folder. If a future local companion is considered, keep it out of scope unless separately approved.

### Backend deployment
Prepare for Railway:
- Bind to `0.0.0.0`.
- Read the listening port from `PORT`.
- Keep secrets in server environment variables or secure server-side settings.
- Use configurable workspace/temp directories.
- Enforce upload size limits and safe cleanup.
- Do not assume ephemeral disk is persistent. Explain what is temporary and what must be downloaded.
- Provide health endpoint and clear startup configuration.
- Never expose server filesystem paths or AI provider secrets to the browser.

## Functionality preservation

Use `FEATURE_CHECKLIST.md` as the definitive feature tracker. Preserve the actual implemented behavior in the source. README claims are useful discovery hints, not proof that every workflow currently works.

Priority feature families:
1. RPF archive open/browse/extract/replace/import/export/rebuild.
2. Mods and assembly mapping/conflict analysis, adapted to uploaded workspaces.
3. Texture dictionaries and embedded textures.
4. Visual settings editing.
5. Particle/YPT editing and batch operations.
6. Model and collision viewing/export.
7. Map/radar layer editing and tile baking.
8. Scaleform/GFX image inspection/replacement/export.
9. Meta/XML editing and binary write-back.
10. Backups, undo/redo, validation, and safe writes.
11. AI assistant/provider integration, with secrets kept server-side.
12. Search, command palette, recent files, presets, and other existing UX.

For every feature, inspect the current source, identify the real functions and UI, map its dependencies, then implement the web equivalent. Do not invent behavior beyond the source without documenting it.

## English conversion

Translate all user-visible Turkish text to natural English:
- Navigation, labels, buttons, dialogs, notifications, tooltips.
- Error and validation messages.
- Settings and onboarding.
- Help text and documentation.
- Newly created API names, types, and identifiers where safe.

Do not blindly rename fields that are part of binary formats, serialization, saved user data, compatibility-sensitive APIs, or Rust crate interfaces. Where internal Turkish identifiers are retained for safety, user-facing text must still be English. Document any intentionally retained identifiers.

## Quality and safety requirements

- No fake buttons, mock processing, placeholder API results, or silent no-ops.
- Validate uploaded file types, sizes, paths, and archive entry paths.
- Prevent path traversal and cross-workspace access.
- Keep each workspace isolated.
- Use atomic writes and validate rebuilt archives before offering downloads.
- Preserve originals and create recoverable backups before mutations.
- Handle malformed and encrypted/unsupported files honestly.
- Surface useful errors without leaking secrets or server paths.
- Add tests for new routes, workspace isolation, upload/download, and core operations.
- Reuse existing core tests; do not weaken or delete them to make a build pass.
- Keep desktop Tauri code intact where feasible.
- Do not change unrelated functionality.

## Dependency and command restriction

**Do not run `npm install`, `npm ci`, `cargo install`, or any dependency installation command without explicit user approval.** Do not update lockfiles or dependency versions just for convenience. Inspect existing manifests and lockfiles first. If a needed dependency is absent, document it and ask for approval before installing or changing dependency state.

You may inspect source, edit files, and run tests/builds using dependencies already present. Report if a command cannot run because dependencies are unavailable.

## Execution order

Follow the milestones in `PROJECT_STATUS.md` and the checklist. Do not skip ahead. Each milestone is a separate approval gate. Do not start a later milestone in the same session, even if the current milestone finishes early.

At minimum:
1. Source audit and feature-to-code mapping.
2. Architecture boundary and web API contract.
3. Minimal vertical slice (upload → open via core → archive tree → export).
4. Migrate frontend API adapter and shell.
5. Convert file/workspace workflows.
6. Migrate editor feature families one by one.
7. Security, validation, backups, and error handling.
8. English translation and UI consistency.
9. Railway deployment configuration and documentation.
10. Full regression verification and final handoff.

Do not create a large untested rewrite in one pass. Keep every milestone coherent and runnable.

## Session handoff protocol

When a session starts, read the living status files and inspect the working tree. Continue from the first incomplete task. Do not redo completed work.

At the end of every milestone, follow `APPROVAL_GATE.md`: prepare a verified full ZIP snapshot, set the stage to READY FOR REVIEW, ask for explicit approval, and STOP.

Before stopping:
- Update checkboxes accurately.
- Record exact changed files.
- Record exact test/build commands and real results.
- List known issues and decisions.
- Write the next precise task in `HANDOFF.md`.
- Ensure another Claude session can continue without asking the user to explain prior work.

## Completion definition

The conversion is complete only when:
- The web app runs locally using documented commands.
- The frontend communicates with the Rust backend.
- Real uploaded RPF files are processed through the existing Rust core.
- Every supported feature has a working web workflow or is clearly documented as an explicitly out-of-scope/unsupported limitation approved by the user.
- No fake functionality remains.
- English user-facing text is complete.
- Upload/workspace security and safe file writes are tested.
- Railway deployment configuration is present and documented.
- Tests and limitations are honestly reported.
- `FEATURE_CHECKLIST.md`, `PROJECT_STATUS.md`, and `HANDOFF.md` accurately reflect final state.

Start by auditing the source and updating the status documents. Do not ask the user to repeat information already in this repository.
