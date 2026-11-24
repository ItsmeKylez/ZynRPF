# M5 REQUIREMENTS: make the web editor work like the reference app (handoff for the next account)
Written 2026-11-24. Status: REQUIREMENTS ONLY, no M5 code exists. Labels: VERIFIED / PARTIAL / UNVERIFIED / SUGGESTED.
The user's words: "I want it to do it like this" (three screenshots of the desktop app tlk-rpf / Zyn-RPF, in `reference_screenshots/`), and "list all of what you need ... I'm giving it to the next account to do". The user deploys to Railway themselves ("just do the code").

## 0. Read first and rules that still apply
1. Read, in this order: `PROJECT_STATUS.md`, `NEXT_SESSION_PROMPT.md`, `TECHNICAL_DECISIONS.md`, `M3_REPORT.md`, `M4_REPORT.md`, `DEPLOY_RAILWAY.md`, then this file. Also re-attach `PRODUCT_SPEC.md` and `TECHNICAL_ANALYSIS.md` (the format facts live there).
2. Do not restart the project, do not regenerate existing code, inspect the repository before changing anything. Preserve decisions D-001..D-045.
3. Clean-room (D-001): the reference app is GPL-3.0. Re-implement features from format knowledge only. Do not copy its code, text, icons or layout pixel by pixel. UI text is US English only (D-002, D-032); the screenshots are Turkish and are only a feature reference.
4. Label every finding VERIFIED / PARTIAL / UNVERIFIED / SUGGESTED. Never claim something works unless it was run. Acceptance N (in-game compatibility) stays "not verified" without real files and an in-game check (D-035).
5. Approval: the user types `APPROVED` or gives a direct instruction such as "just do the code". Ask one short question if the scope of an approval is ambiguous. Each phase below needs its own go-ahead. Do not touch Railway; the user deploys.
6. Update `PROJECT_STATUS.md`, `NEXT_SESSION_PROMPT.md`, `TECHNICAL_DECISIONS.md` after each phase.

## 1. Current state (VERIFIED in the previous account's container, 2026-11-24)
- Repository `rpf-editor` (latest package `rpf-editor-m4.zip`): Rust `rpf-core` (parser, rebuild, validate), Rust std-only HTTP `server`, React/TypeScript `web/`.
- M1 backend, M2 web UI (browse, search, entry details, read-only hex), M3 edit and save (rename, delete, new folder, add file, replace stored contents; undo/redo; one writer; save as new validated version; versions cap 10), M4 packaging (Dockerfile, entrypoint, railway.json, `GLOBAL_QUOTA_BYTES`).
- Tests that passed: `cargo test` (11), `scripts/e2e.mjs` 70/70, `scripts/e2e-m3.mjs` 78/78, `scripts/e2e-m4.mjs` 6/6, `scripts/ui-test-m3.cjs` 23/23, English audit. Fixture generator: `cargo run -p rpf-core --example gen -- <out> <entries> <bytes>`.
- The user started deploying on Railway: the Docker build ran; two start problems were fixed (Root Directory must be the folder holding the Dockerfile; entrypoint exec bit and CRLF now handled in the Dockerfile). Whether the app then ran healthy is UNKNOWN: ask the user for their deploy result.
- NOT verified: any real GTA V / FiveM archive, Docker image build in the test container, UI `tsc`/`vite build` and axe scan (no network or `@types/react` there), other browsers.
- Temporary deviations still in force (D-039): no sandbox for parsing, in-memory state (wiped on restart; one instance only), no accounts/Postgres/backups, std-only code with no third-party crates.

## 2. What the user wants, read from the three screenshots (VERIFIED from the images; the app is Turkish)
**Screenshot 1, start screen:** left panel "tlk-rpf / FiveM mod editor" with buttons "Open RPF" and "New mod", a "Mods folder" section (refresh, open folder, history, edit icons; shows "FiveM mods folder not found" with a Windows path), a centre drop zone "Drop a .rpf or pick a mod on the left. Once opened, editors appear automatically based on the files inside: textures, visual settings, text files and ready presets." with buttons Open RPF, New mod, Quick search, a "Recently opened" list (file name plus folder), and a shortcut bar: Ctrl+P quick search, Ctrl+K AI, Ctrl+S save, Ctrl+Z undo.
**Screenshot 2, summary page of an opened archive:** a tab per archive; header with the package name ("Tracer Bloodfx"), file name, tag ("VLSHOP"), version ("v2.5"), encryption badge ("OPEN"), size; buttons AI, Undo, Redo, open-folder, "Save as", "Save" (disabled until changed). Left: file search box with expand-all and collapse-all/export icons, and a tree (content > folders > nested `ptfx_hi.rpf` which expands inside the tree > files; sizes at right). Right: "Summary and ready settings" page: counts (6 files, 1 dictionary, 2.8 MB on disk, about 2.8 MB when saved); an orange "Needs fixing" box (files in the package are not matched to a game path, so they will not go into the game; fix with "Add target file"); "Ready settings" (presets; none found); "Textures and visual settings" (open texture dictionaries, replace images by dropping them; lists `.../ptfx_hi.rpf/core.ypt` 2.8 MB); "Other files" (text files open in the editor, others can be exported or replaced); "Package (assembly.xml)" form (package name, author, version) with "Add target file" and "Extract all".
**Screenshot 3, particle effect editor for `core.ypt`:** tabs "Effect settings" and "Textures"; summary "811 effects, 1812 emitters, 9066 particle settings"; search box; filter chips All / effect / emitter / particle and a colour-only checkbox; a long list of effect cards (for example `bul_gravel_heli`), each with named values such as "Effect colour tone (max)" and "Effect scale" shown as keyframe rows (time and value); a right panel "Bulk edit (11689 results)": Hue shift slider, Colour picker, Size x, Light x, Density x, and an "Apply to filtered" button; note "Density 0 means the effect makes no particles (performance mode)"; a "How it works" note: each value is a keyframe (small number = moment in the effect's life, 0 start and 1 end); changes wait, the top-right "Apply" writes them in one step; Ctrl+Z undoes.

## 3. Feature list for the web version (SUGGESTED IDs; priorities are SUGGESTED)
| ID | Feature | Source | Priority | Notes and dependencies |
|---|---|---|---|---|
| F-01 | Tabs: several archives open at once, one tab each | S2 | High | Today: one workspace, one active archive. Needs `MAX_WORKSPACES` > 1 or tabs for versions; check quotas (D-029). |
| F-02 | Header: package name, file, tag, version, encryption badge, size | S2 | Medium | Name/version come from `assembly.xml` when present (F-12); otherwise file name. Badge from existing encryption tag. |
| F-03 | Undo, Redo, Save, Save as (download a copy), buttons enabled only when relevant | S2 | High | Undo/redo and Save exist (M3). Add Save as = save then download; keyboard Ctrl+Z, Ctrl+S (and Shift+Ctrl+Z). |
| F-04 | Expandable file tree with search, expand all, collapse all, export all | S2 | High | Today a flat folder list plus separate search. Tree needs lazy loading (1M entries). "Export all" = zip stream of the archive contents (needs a zip writer, std-only). |
| F-05 | Nested RPF archives open inside the tree (ptfx_hi.rpf expands) | S2 | High | Needs nested parse by byte range from the parent (no copy), depth limit 4 (D-008). Editing inside nested RPF requires rebuild of the nested archive then the parent (M3 listed this as out of scope). |
| F-06 | Live preview of pending edits in the tree | gap found in M3 | High | Today the tree shows the base version until Save. Serve the tree from the replayed `Tree` (rpf-core already builds it) or render pending ops client-side. |
| F-07 | Summary page: counts, sizes, estimated size after save, "Needs fixing" warnings, presets, textures, other files lists | S2 | High | Estimated size = `Tree::plan().file_len` (exists). Warnings need a rule set (see F-13). |
| F-08 | Quick search (Ctrl+P) across the archive and nested RPFs | S1 | Medium | Search API exists for one level; needs nested scope. |
| F-09 | Text editor for text files (xml, txt, meta, lua, json) | S2 | High | Files are often compressed in real RPFs, so needs inflate and deflate (section 4). Saving writes the file as a replacement op. Show size, encoding detection, line endings; limit size. |
| F-10 | Texture dictionary editor (YTD): list textures, preview, replace image by dropping a file | S2, S3 | High | Needs RSC7 + YTD parse (TECHNICAL_ANALYSIS s2). Replace requires a DDS with matching format or a re-encode; see section 4. |
| F-11 | Particle effect editor (YPT): list effects/emitters/particles, search/filter, per-value editing, bulk edit (hue, colour, size, light, density), "Apply to filtered", undo | S3 | Medium-High | Hardest item. Needs the RSC7 pointer graph and the particle rule structure (UNVERIFIED, no real sample). Edit in place without changing sizes where possible so no re-layout of the resource is needed. |
| F-12 | Package editor (`assembly.xml`): name, author, version fields, "Add target file", "Extract all" | S2 | Medium | Schema of `assembly.xml` is UNKNOWN (only seen as 974 B file in the screenshot). Ask the user for a sample. |
| F-13 | "Needs fixing" check: files not mapped to a game path | S2 | Medium | Rules come from the assembly format; define after F-12. Register new validation codes append-only (D-031). |
| F-14 | Presets ("Ready settings") | S1, S2 | Low | Meaning unknown (maybe prepared edit packs per content type). Ask the user. |
| F-15 | Start screen: drop zone anywhere, Open RPF, New mod, recent files list | S1 | Medium | "Recent" = session only (anonymous sessions, D-015). "New mod" = create a new empty RPF (needs `rpf-core` writer for an empty archive; exists as rebuild of an empty tree). Mods folder: NOT possible in a browser (D-003). |
| F-16 | Move and copy entries; drag-and-drop in the tree | gap | Medium | Not in M3. New ops (append-only log, undo/redo, tests). |
| F-17 | Replace contents of compressed and resource entries; compress new data | gap | Medium | Needs deflate; resource replace for YTD/YPT only through their editors (F-10/F-11). |
| F-18 | Hex viewer: keep read-only; optional "Patch bytes" op | D-017 | Low | Open question 5; default off. |
| F-19 | Drop (not building): AI assistant, Ctrl+K, mods folder scan/toggle/install, file lock, open-folder button | S1, S2 | Rejected | D-003. Say so in the UI nowhere; just omit. |

UI quality asks (SUGGESTED): dark theme like the screenshots with a light/dark switch; wider two-pane layout (tree left, editor right); keyboard shortcuts with a visible shortcut bar; keep WCAG 2.1 AA (D-018); English only.

## 4. Technical needs and blockers (what the next account must resolve)
1. **Compression.** Inflate and deflate are needed for F-09, F-17 and for reading compressed resources (YTD/YPT payloads are deflate-compressed inside RSC7). No third-party crates were available offline (std-only). Options: (a) write inflate (about 200 lines) and a simple deflate (stored or fixed-Huffman) in `rpf-core`, tested against `zlib` via Python in tests; (b) if the new account has network, use `flate2` (MIT/Apache-2.0, passes D-028) with `cargo deny`. Decide and record as a decision.
2. **Encrypted (NG/AES) archives.** Real GTA V archives are usually encrypted; D-004 says detect and report only. The user's screenshot shows an "OPEN" archive (FiveM mods are usually OPEN). Keep detect-only unless the user changes D-004.
3. **RSC7 resources (YTD, YPT).** Flags, virtual/physical page sizes, pointer fix-ups, large-resource marker 0xFFFFFF (parser already handles the marker). Facts are in TECHNICAL_ANALYSIS s2 (UNVERIFIED in-game, D-011). A resource editor needs: decompress, parse the graph, edit, re-serialise with correct flags, recompress, write back as a Res entry.
4. **Textures.** DDS read (header, BC1/BC3/BC5/BC7 sizes, mip chain), validation of dimensions and format versus the target texture, optional re-encode (the spike for `intel_tex_2` was blocked: needs network and a Windows/Linux build check). Safe first version: replace only with DDS files of the same format, show errors otherwise.
5. **Particles (YPT).** Needs the ptfx rule/effect/emitter/particle structure. UNVERIFIED and large (811 effects, 9066 settings in the screenshot). Without a real `core.ypt` this cannot be built responsibly. Plan: parse read-only first, show the counts, then in-place float edits.
6. **Server model.** The editor panels need new endpoints (resource summaries, text read/write, texture list, bulk edit). Keep Range reads, limits, rate limits, one-writer rule, validate-before-publish for every saved version. A resource edit becomes a new op type in the log (`Op::ReplaceResource` or similar) so undo/redo still works.
7. **Sandbox.** Parsing and rebuilding resources of untrusted files in the server process is risky (D-039). Do the sandboxed worker (D-024) before any public launch, and strongly before resource parsing ships publicly.
8. **Performance.** Tree with lazy nodes for 1M entries (measured earlier: 1M-entry browse, parse 97 ms; replay of ops about 0.3 to 0.4 s). A full save timing on 1M entries was not completed.
9. **Frontend build.** `tsc`/`vite build` were never run in the test container (no network). First step for the next account if it has network: `cd web && npm ci && npm run build`, fix type errors, run `scripts/ui-test.cjs` (M2 suite, needs `AXE_PATH` to axe-core) and `scripts/ui-test-m3.cjs`.

## 5. What the next account needs from the user
1. **At least one real sample archive** (the user's `530.rpf`, an OPEN FiveM RPF with a nested `ptfx_hi.rpf` / `core.ypt`, `bloodfx.dat`, `assembly.xml`) and ideally a plain `.ytd` and one text-heavy RPF. Without them F-10 and F-11 stay UNVERIFIED. They stay outside the repository (D-030). The user did not answer this question; assumption made: no sample yet, synthetic fixtures only.
2. The user's real **`assembly.xml`** (and what "Add target file" and "Ready settings" mean).
3. Answers to the open questions in `TECHNICAL_DECISIONS.md` (encryption D-004, hex patching D-017, quotas, accounts D-015, upload ceiling) and confirmation of D-044 and D-045.
4. The **Railway deploy result** (build log or "it works"); their hosting plan limits (memory, volume size).
5. Optional: re-attach `Zyn-RPF-main.zip` only if a format fact must be re-checked (read for facts, never copy code).
6. Which of F-01..F-18 matters most and which can be cut. The assistant's SUGGESTED order is in section 6.

## 6. Suggested phases (each needs the user's go-ahead; sizes are SUGGESTED, not measured)
- **M5a Workspace UX (F-01, F-03, F-04, F-06, F-07, F-08, F-15 minus mods folder):** new layout, tabs, lazy tree, live pending-edit preview, summary page, Save as, shortcuts, start screen. No new format work. Exit: browser tests for each, English audit, axe scan, M1-M4 regressions pass.
- **M5b Compression and text editing (F-09, F-17 partial, F-02):** inflate/deflate with test vectors, text editor, compressed entries replaceable, nested RPF open (F-05 read) then nested edit. Exit: round-trip tests with real zlib data, fuzz of the inflater, size limits and zip-bomb ratio check (D-008).
- **M5c Package and fixes (F-12, F-13, F-14):** after the user supplies `assembly.xml`. New validation codes.
- **M5d Textures (F-10):** RSC7 + YTD parse, preview, same-format DDS replace. Needs a real `.ytd`.
- **M5e Particles (F-11):** read-only parse, then in-place edits and bulk edit. Needs a real `core.ypt`.
- **M5f Hardening before public launch:** sandboxed worker (D-024), persistence decision, npm licence gate (D-042), load test, real-archive compatibility pass, in-game check by the user.

## 7. Known weaknesses of the current build the next account should know
- Pending edits are not shown in the tree until Save (F-06).
- No move/copy; no new empty archive; no nested RPF; no compressed entry replace; resource entries cannot be replaced (EDIT_RESOURCE_UNSUPPORTED).
- One workspace per session by default (`MAX_WORKSPACES=1`).
- Entry order inside a folder is kept, new entries go last; the real game may expect another order (PARTIAL).
- Large-resource payload length is inferred as the distance to the next payload.
- Rate limits and quotas are untuned guesses; writer heartbeat uses one write request every 10 s per open tab.
- Fuzzing of the rebuild path is 400 sequences, not CI scale (D-010).

## 8. Test and quality bar for every M5 phase
Round-trip (parse, edit, save, parse, compare), untouched bytes identical, hostile inputs with stable check codes (append-only registry), interrupted save leaves the previous version intact, browser tests in Chromium, English audit, axe scan, keyboard operation, regression of `e2e.mjs`, `e2e-m3.mjs`, `e2e-m4.mjs`, `ui-test-m3.cjs`. Report honestly what was not run.

## 9. Risks
Real-game compatibility (high), resource format correctness without samples (high), unsandboxed parsing of hostile files (high for public use), memory use for big resources (medium), scope size: the reference is a desktop app with 60 commands (medium), clean-room discipline with a GPL reference (medium).
