# M5a REPORT: Workspace UX, first slice (2026-11-24)
Scope: the user replied `APPROVED` to two questions (fix the stale NEXT_SESSION_PROMPT; make M5a the next stage) and asked for it to be done before the usage limit. Because of that limit this is a frontend-only slice of M5a. The Rust backend was not changed.

## Built (web/src)
- F-04 partial: lazy file tree (`TreeView.tsx`): folders load children only when expanded, 200 per page with "Show more", "Collapse all", selecting a file or folder selects it for editing and details. Sits above the existing grid, which stays.
- F-07 partial: Summary panel (`Summary.tsx`): file name, size on disk, entry count, pending changes, estimated size after saving (shown after "Check changes", from the server plan).
- F-03: keyboard shortcuts Ctrl/Cmd+Z, Ctrl/Cmd+Shift+Z or Ctrl+Y, Ctrl/Cmd+S (Z and Y are ignored inside text fields so native text undo still works); visible shortcut list; "Save and download" button (save as a new version, then download it).
- Theme switch (Automatic, Light, Dark). Offline bundle script `scripts/build-web-offline.sh`; new test `scripts/ui-test-m5a.cjs`.

## VERIFIED in this container (Chromium, synthetic 6 MiB archive)
- `scripts/ui-test-m5a.cjs`: 16 of 16 pass (tree lazy load, collapse all, selection, summary, estimate after check, Ctrl+Z, Ctrl+Shift+Z, Ctrl+S, Save and download gives a `7FPR` (RPF7) file, theme, shortcut list, no page errors).
- `scripts/ui-test-m3.cjs`: 23 of 23 pass after one selector change in the test (the new theme select made `select` ambiguous).
- English audit: pass (197 catalogue keys). `cargo test --offline`: 11 pass.

## NOT done / NOT verified
- Not in this slice: F-01 tabs, F-05 nested RPF, F-06 live preview of pending edits in the tree (needs a backend change), F-08 quick search Ctrl+P, F-15 start screen and recent list, Expand all, Export all, header badges (name, tag, version, encryption), Save as with a custom file name.
- Not run: `tsc` type check and `vite build` (no @types/react and no network; the bundle was made with a local esbuild and React 19, the project pins React 18.3.1), axe scan, M2 browser suite, e2e API suites (backend unchanged), other browsers, Railway, Docker.
- Real game files: you attached `Zyn.rpf`, `core.ypt` and `bloodfx.dat`. They were only identified by header (RPF7, RSC7 marker). They were not parsed or tested, so acceptance N is still not verified.
- The keyboard Ctrl+S overrides the browser's save-page shortcut while the editor is writable.

## Decision proposed
D-046: M5a delivered as a frontend-only slice because of the usage limit; the remaining M5a items stay open. PROPOSED, needs your confirmation.

## Recommended next step (needs your `APPROVED`)
Finish M5a with the backend part: serve the tree from the replayed edit state (F-06), then F-08 and F-15. Or, if you prefer, start M5b (inflate/deflate and text editor) and test it on your real `Zyn.rpf`.

---
# Part 2: live preview of pending edits (F-06), 2026-11-24
Scope: the user replied `APPROVED` to the part-1 report, which offered two options. Read as "finish M5a" (the stage in progress), not M5b. Because of the usage limit only F-06 was done; F-08 and F-15 were not.

## Built
- rpf-core: `Tree::list_folder(path)` returns the children of a folder after the pending edits (name, folder or file, size, new flag), sorted by name.
- Server: `GET /api/v1/workspaces/{id}/edit/tree?version=&path=&cursor=&limit=` replays the pending operations on the base version and lists one folder (read-only; no writer lease needed; limit 1 to 500). Additive; no existing route changed.
- Web: while changes are pending, the tree is served from that endpoint, shows the note "The tree shows your pending changes. They are not saved yet.", and items can be selected for rename, delete and replace by path. Files added by you carry a "new" badge.

## VERIFIED in this container (synthetic archives, Chromium)
- ui-test-m5a: 20 of 20 (now includes: pending folder appears in the tree, can be selected, pending delete removes it, undo brings it back).
- ui-test-m3: 23 of 23. English audit: pass (199 keys). cargo test: 11 pass.
- API suites with their documented settings: e2e (M1) 70 of 70, e2e-m3 78 of 78. (They first failed under default server settings; the scripts need the environment listed in their headers.)

## Limits
- New folders do not get the "new" badge (only added files do). Renamed items show their new name without a badge.
- Each change reloads the tree and collapses expanded folders. Every reload replays the pending operations on the server (about 0.3 to 0.4 s on 1,000,000 entries, measured earlier; not re-measured for this route).
- The grid below the tree still shows the base version until Save.
- Not run: tsc, vite build, axe scan, M2 browser suite, real game files, Docker, Railway.

## Still open in M5a
F-01 tabs, F-05 nested RPF, F-08 quick search (Ctrl+P), F-15 start screen and recent list, Expand all, Export all, header badges, grid preview.
D-047 (PROPOSED): preview endpoint as described above.

---
# Part 3: quick search (F-08, one level) and drop anywhere (F-15 partial), 2026-11-24
Scope: the user replied `APPROVED` again to the part-2 report, which asked which step comes next. Read as: keep finishing M5a (consistent with part 2). Only the smaller open items were done; backend untouched in this part.

## Built (web only)
- Quick search dialog (Ctrl/Cmd+P, or the "Quick search" button): type to search the whole open archive (existing search API, 200 ms debounce), Up/Down to move, Enter to jump to the result, Escape to close, focus returns to where it was. It overrides the browser's print shortcut on the editor page.
- Drop a file anywhere on the page to open it (a full-screen overlay shows while dragging); the old drop zone still works and does not double-fire.
- Shortcut list now includes Ctrl+P. Modal styles added.

## VERIFIED in this container (synthetic archives, Chromium)
- ui-test-m5a: 24 of 24 (adds: Ctrl+P opens with focus in the box, lists matches, Enter picks and closes, Escape closes).
- ui-test-m5a-drop (new): overlay appears on drag, drop on the page header opens the archive.
- ui-test-m3: 23 of 23. English audit: pass (207 keys). Server and Rust core unchanged since part 2 (its API suites passed there).

## Limits
- Quick search covers the open archive only, not nested RPFs (no nested RPF support yet, F-05). The dialog is a simple one: Tab keeps focus in the box; it was not checked with a screen reader or the axe scan.
- Drop anywhere was tested with synthetic drag events, not with a real operating-system drag.
- "Recent files" is not built (the version list already shows what is open in the session).
- Not run: tsc, vite build, axe, M2 browser suite, real game files, Docker, Railway.

## Still open in M5a
F-01 tabs, F-05 nested RPF (belongs with M5b), Expand all, Export all, header badges (F-02, belongs with M5b), recent list, grid preview of pending edits.

---
# Part 4: Expand all (bounded), 2026-11-24
Scope: the user replied `APPROVED` to a three-option question (finish M5a / start M5b / other). Read as option 1, finish M5a (same reading as parts 2 and 3). M5b was NOT started. This container has no Rust toolchain and no running server, so only a small web change was made.

## Built (web only)
- "Expand all" button beside "Collapse all". It opens folders down to 3 levels, 200 items per folder (lazy tree; opening a whole 1,000,000-entry archive would be unsafe). A note explains the limit.
- Test added to `scripts/ui-test-m5a.cjs` (expand all, then collapse all).

## VERIFIED
- `scripts/build-web-offline.sh` bundles without errors. English audit: see run result in the session.

## NOT verified
- The new test was NOT run (no cargo, so no server; the browser suite needs one). tsc, vite build, axe, real game files unchanged from part 3.

## Still open in M5a
F-01 tabs (needs more than one workspace, backend quota check), Export all (needs a std-only zip writer in the server), recent list, grid preview. F-05 and F-02 belong with M5b.

---
# Part 5: grid note, 2026-11-24
Scope: the user replied a bare `APPROVED` to a question asking which step comes next (tabs, Export all or M5b). Not specific; read as "you choose", narrowest reading: stay in M5a, web-only. M5b not started.
- Full grid preview of pending edits was NOT built: the grid is keyed by saved entry ids and feeds details and hex, so a rewrite needs a running server to test, and this container has no Rust toolchain.
- Built instead: a note above the grid while changes are pending ("This list shows the saved version ...").
- VERIFIED: English audit passes, offline bundle builds. NOT run: any browser test, tsc, axe, real files.
- Still open in M5a: F-01 tabs, Export all, recent list, real grid preview. Tabs and Export all need backend work and a Rust machine to verify.

---
# Part 6: one command to run every check, 2026-11-24
Scope: the user said "I APPROVED WHAT YOU CHOOSE". Chosen: no new Rust code, because this container has no Rust toolchain (checked twice) and untested server code could break the user's build. Instead: `scripts/run-all-checks.sh`, which builds the web UI if needed, generates the fixtures, starts a fresh server with each suite's documented settings and runs cargo test, e2e (M1), e2e-m3, e2e-m4, ui-test-m3, ui-test-m5a (includes Expand all), ui-test-m5a-drop and the English audit.
- VERIFIED: shell syntax only (`sh -n`). The script itself was NOT run (needs cargo). The server's WEB_DIR and PORT settings were read from main.rs and api.rs. Expect to fix small things (for example Playwright resolution, the quota values for the ui suites) on first run.
- Purpose: runs everything from parts 4 and 5 that is still untested. If anything fails, send the `out-*.log` files named in the output.

---
# Part 7: read-only look at the real Zyn.rpf, 2026-11-24
Scope: user said "you choose" again. Still no Rust toolchain here, so chosen: a throwaway read-only Python read of the 16-byte header of `Zyn.rpf` (no project code changed).
- VERIFIED from the file bytes: magic `7FPR` (RPF7), entry count 10, names-table length 144 bytes, encryption tag `OPEN` (0x4E45504F, i.e. not encrypted), file size 3,065,856 bytes. `core.ypt` starts with `RSC7`; `bloodfx.dat` is plain text.
- CORRECTION (PARTIAL): earlier notes said Zyn.rpf has "144 entries". The header says 10 entries; 144 is the names-table length. The earlier figure was probably read from the wrong field. Please treat "144 entries" as wrong.
- My quick decoding of the entry table was a guess and did not match RPF7's real entry layout (it gave nonsense offsets), so nothing beyond the header is claimed. The real test is rpf-core parsing this file: after you run `run-all-checks.sh`, try opening Zyn.rpf in the app and tell me what it shows (expect 10 entries).
