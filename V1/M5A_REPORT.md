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
