## Checkpoint ZIPs during a stage

The user must receive files before a free usage limit can interrupt work. Follow `CHECKPOINT_AND_USAGE_LIMIT_PROTOCOL.md`.

A checkpoint ZIP is required:
- Before substantial implementation (initial snapshot).
- After each meaningful batch, normally 3–5 changed files or one independently testable unit.
- Before a lengthy or risky operation.
- Immediately when a usage-limit warning appears or the session is likely to end.

A checkpoint is an in-progress recovery artifact, not a completed milestone. Label it `IN-PROGRESS CHECKPOINT — NOT A STAGE RELEASE`. Continue only within the currently approved stage. The normal stage-completion ZIP and explicit approval gate remain mandatory.

# APPROVAL GATE — MANDATORY USER CONTROL

This document overrides any instruction that could be interpreted as permission to continue automatically.

## Absolute rule

**You must stop after every stage and wait for the user's explicit approval before beginning the next stage.**

The user owns the project direction and must approve each milestone. Silence, continued conversation, a request for a ZIP, or a request to explain progress is NOT approval to continue.

Only clear statements such as "I approve Stage 1", "Approved, continue to Stage 2", or equivalent explicit authorization count as approval.

If the user asks for changes, corrections, explanations, or a revised ZIP, do only that work within the current stage. Do not advance to the next stage until the user explicitly approves the completed stage.

## Stage workflow

For each stage:

1. Work only on the currently approved stage.
2. Implement the stage's agreed scope; do not start future-stage work "while you're here".
3. Run available relevant tests/checks without installing dependencies or changing dependency state unless the user has explicitly approved that.
4. Update `PROJECT_STATUS.md`, `FEATURE_CHECKLIST.md`, and `HANDOFF.md`.
5. Create a complete ZIP snapshot of the project, including source and all handoff documents, excluding generated build caches, secrets, user data, and dependency folders where appropriate.
6. Verify that the ZIP exists and can be opened/listed.
7. Present the ZIP and a concise report:
   - Stage number and name.
   - What was implemented.
   - Files changed.
   - Tests/checks actually run and their results.
   - Known limitations or incomplete items.
   - What the next stage will do (description only; do not start it).
8. Ask the user to review and explicitly approve the next stage.
9. **STOP. Do not make any further project changes until approval arrives.**

## Approval states to track

Add an "Approval Gate" section to `PROJECT_STATUS.md` and keep it current:

- Current stage:
- Current stage status: NOT STARTED / IN PROGRESS / READY FOR REVIEW / CHANGES REQUESTED / APPROVED
- User approval received for this stage: YES / NO
- Next stage:
- Permission to begin next stage: YES / NO

Never set permission to begin the next stage to YES unless the user explicitly approves it in the conversation.

A ZIP handoff does not equal approval. A new Claude account must read these files and continue to wait if the next stage has not been approved.

## Stage boundaries

Use the milestones in `PROJECT_STATUS.md` as the approval stages. If a milestone is too large to review safely, split it into smaller numbered sub-stages and ask the user to approve each one. Do not combine multiple milestones into one delivery just to move faster.

The first stage is M0 — Source Audit. Its deliverable is the source inventory, command-to-core mapping, conversion risks, and baseline test/build report. It must not start implementation of the web backend or change application architecture before the user approves M0 and the next stage.

After M0 approval, proceed to M1 only. After M1 approval, proceed to M2 only, and so on.

## If the user switches Claude accounts

The next Claude account must:
1. Read `CLAUDE_START_HERE.md`, `APPROVAL_GATE.md`, `PROJECT_STATUS.md`, `FEATURE_CHECKLIST.md`, and `HANDOFF.md`.
2. Inspect the working tree and the latest ZIP state.
3. Check the Approval Gate section.
4. If permission to begin the next stage is NO, do not start that stage. Wait for the user.
5. If a stage is marked READY FOR REVIEW or CHANGES REQUESTED, work only on the user's requested review corrections; do not advance.

The next account must never infer approval from a previous assistant's claim. The user's explicit approval is required.

## What counts as stage completion

A stage may be marked READY FOR REVIEW only when:
- Its stated deliverables are complete.
- Relevant checks have been run where possible.
- Known gaps are documented.
- The project is in a coherent state.
- The handoff documents are updated.
- A review ZIP is prepared and verified.

If a stage cannot be completed, report the blocker and ask the user how to proceed. Do not silently skip it or mark it complete.
