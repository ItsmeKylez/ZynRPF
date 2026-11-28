# CHECKPOINT & USAGE-LIMIT DELIVERY PROTOCOL — MANDATORY

This protocol exists because work must not be lost when Claude reaches a free usage limit, session cutoff, context limit, or account cooldown.

## The most important rule

**Do not wait until a stage is finished to send the user a file. Send a downloadable, recoverable project ZIP early and repeatedly while work is in progress.**

The user explicitly wants the latest project file delivered before Claude runs out of free usage. Treat file delivery as a continuous requirement, not a final-stage task.

Claude cannot reliably predict the exact moment a free usage limit will occur and must never claim that it can. Therefore, use conservative checkpoints and act immediately whenever a limit warning or interruption risk becomes visible.

## Required delivery schedule

1. **Starting snapshot:** As soon as the project has been opened and its current state verified, create and send a ZIP snapshot before beginning substantial edits. This gives the user a recoverable baseline.
2. **Small work batches:** After each meaningful, self-contained batch (normally no more than 3–5 files changed, or one independently testable unit), save the files and update the checkpoint records.
3. **Regular downloadable checkpoints:** Create and send a full project ZIP after each meaningful batch. Do not wait for a milestone to finish.
4. **Before risky or lengthy work:** Send a ZIP first, then begin the operation.
5. **Usage-limit warning:** If Claude displays, reports, or otherwise makes clear that usage is running low, STOP feature development immediately. Save all current work, update the status and handoff documents, create and verify a ZIP of the latest working state, and send that ZIP to the user in the same response if file attachment is supported.
6. **At session end:** Never end with only a progress report if a project file can be packaged. Deliver the latest ZIP and state exactly what is and is not included.
7. **After a cutoff/restart:** The next session must inspect the latest working tree and checkpoint documents, compare them with the most recent delivered ZIP, and continue from the newest verified state. Never assume a ZIP contains changes that were not packaged.

## What every checkpoint must contain

- The complete project source as it currently exists.
- All project-control documents, including this protocol.
- Updated `PROJECT_STATUS.md`, `FEATURE_CHECKLIST.md`, and `HANDOFF.md`.
- A checkpoint manifest in `CHECKPOINT_LOG.md` recording:
  - Checkpoint ID and date/time (local timezone if known).
  - Current stage and approval state.
  - Summary of included work.
  - Files changed since the previous checkpoint.
  - Tests/checks run and actual outcomes.
  - Known bugs, incomplete work, and blockers.
  - Exact next action.
- No secrets, API keys, credentials, private user data, build caches, or unnecessary dependency folders.
- Do not omit source files merely to make the ZIP smaller unless they are generated or explicitly excluded by the repository's ignore rules and are reproducible.

## ZIP creation and verification

For every checkpoint:
1. Package the latest complete working project, not just the changed files.
2. Use a clear versioned filename, for example:
   `Zyn-RPF-Checkpoint-M2-CP003.zip`
3. Verify that the ZIP exists, is non-empty, and can be opened/listed.
4. Confirm that key source files and all handoff documents are inside it.
5. If possible, calculate and record a SHA-256 checksum.
6. Attach/provide the actual downloadable ZIP to the user. Never provide a guessed link or claim delivery if no attachment was created.
7. Keep the previous checkpoint ZIP until the new one has been verified.

## Approval gates still apply

Checkpoint delivery is NOT stage approval.

- Claude may create and send checkpoint ZIPs during the currently approved stage.
- Claude must not begin the next stage without the user's explicit approval.
- A checkpoint ZIP can be incomplete and must be labeled `IN-PROGRESS CHECKPOINT — NOT A STAGE RELEASE`.
- A stage-complete ZIP must be labeled `READY FOR REVIEW`.
- Do not change the approval state just because a checkpoint was delivered.

## If there is not enough time/usage to finish packaging

Prioritize in this order:
1. Save all modified source files.
2. Ensure files are not half-written or syntactically truncated.
3. Update `PROJECT_STATUS.md` with exact state.
4. Update `HANDOFF.md` with the exact next action.
5. Update `CHECKPOINT_LOG.md`.
6. Create and verify the ZIP.
7. Send the ZIP.
8. Only then provide a brief explanation.

Do not spend the remaining usage writing a long explanation while unsaved work remains.

## Honesty and capability limits

- Do not claim to know the exact remaining free usage unless the interface explicitly provides that information.
- Do not promise a delivery at a precise percentage of usage that Claude cannot observe.
- Use conservative, frequent checkpoints so the user receives the file well before a possible cutoff.
- If the environment cannot create or attach a ZIP, say so immediately and provide the exact Git commit/hash or file-saving location that preserves the work. Do not pretend a ZIP was attached.
- If a hard cutoff happens before a checkpoint can be sent, the latest saved working-tree state and Git commit are the recovery source; this is why saving and committing continuously is mandatory.
