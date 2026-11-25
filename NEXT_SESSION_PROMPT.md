# NEXT SESSION PROMPT (handoff 2026-11-25, after M8a). Follow the master prompt too.

## What the app does now
Web RPF editor (Rust server `crates/server` + `crates/rpf-core`, React/TS in `web/`): open/browse/search RPF7 archives incl. nested, edit/add/delete/rename/move with undo/redo, text editor, RSC7 header panel, save + validate + download + ZIP export, PARTICLE (.ypt) editor, VISUALSETTINGS.DAT editor, and now the TEXTURE editor (M8a).

## Done this session
- `crates/rpf-core/src/ytd.rs`, `doku.rs`: ORIGINAL core files copied verbatim (header comment only). New deps in rpf-core: image, fast_image_resize, intel_tex_2, bcdec_rs, texpresso, rage-formats. `crates/rpf-core/build.rs` links libstdc++ (intel_tex_2 needs it). Dockerfile now installs g++ (build) and libstdc++6 (runtime) - NOT yet built in Docker.
- `crates/server/src/api/textures.rs`: GET `.../entries/{eid}/textures` (list), `.../textures/{i}/png?mip=`, `.../textures/{i}/dds`, POST `.../textures/{i}/replace?fit=stretch|fit|fill&hue&saturation&brightness&contrast&tint&tint_strength` (body = PNG/JPEG/WebP/BMP/TGA/DDS, max 32 MiB, max 8192 px side), POST `.../textures/recolor?indices=1,2&...`. Replace/recolor return the edited resource bytes; the UI queues them as a Replace op. Texture keeps its size, format and mip count. Works for .ytd and for dictionaries embedded in .ydr/.ydd/.yft/.ypt.
- `web/src/TextureEditor.tsx`: lazy thumbnails, search, big preview with mip selector, export PNG/DDS, replace with image + fit mode, color adjustment (hue, saturation, brightness, contrast, tint) for the selected or checked textures.
- Stacking fix: `?base_blob=<id>` on the ytd/vs/ypt read and edit routes makes them work on a queued (unsaved) replacement. Panels remember the blob id after queueing, so a second edit no longer drops the first and the panels show queued changes. After queueing, Browser switches the grid to the saved view (`setGridPreview(false)`) so the selected entry stays selected. Resets when the pending list becomes empty (`pendingOps` prop) or the entry/version changes.
- CSP now `img-src 'self' blob:` (needed for previews; found by the browser test).

## Tested (actually run)
`sh scripts/run-all-checks.sh`: ALL PASSED, including new e2e-ytd (36 checks: list, PNG per mip, DDS, bad input, recolor + replace + save + reopen, base_blob stacking), ui-test-ytd (9 checks in Chromium: thumbnails, preview, mip, recolor, replace, stacked edits, save, reopen, export), plus all earlier suites.
Real file: user's Zyn.rpf -> ptfx_hi.rpf -> core.ypt: 106 embedded textures listed (DXT1, DXT5, A8R8G8B8, none unsupported), PNG preview, recolor of one texture returned a 2.9 MB resource and the preview with base_blob worked (read-only check; the real archive was not saved/reopened for textures).

## NOT tested
Saving a real recolored core.ypt and reloading it; real standalone .ytd; BC7/other formats on real data; FiveM/GTA loading edited archives; Docker build (g++/libstdc++ lines new); Railway; axe scan; the ypt "Dokular" bulk UI; base_blob stacking for visualsettings/ypt panels in a browser (only the API path and texture UI are tested).

## Known problems / limits
Undoing the queued op in the Edit panel while the editor panel still holds its base blob can resurface the undone change on the next edit (base only resets when the pending list is empty). Recolor re-encodes from decoded pixels (lossy for DXT). Replace output is always the existing texture's size/format. Preview limited to 16M pixels per level. Textures of unsupported formats are listed but cannot be previewed.

## Exact next task
STOP and wait for the user's `APPROVED`. Suggested next milestone M8b (needs APPROVED): add / delete / rescale textures in a .ytd (already in the verbatim ytd.rs: `yeniden_kur(dosya, Vec<DokuIslemi>)` for add/delete, `boyutlandir` for rescale, `yeniden_adlandir` for rename; the original Tauri command is `ytd_islem` in src-tauri/src/lib.rs), plus selecting a new format when adding; embedded dictionaries stay replace/recolor only. Then other stages from docs/NEXT_SESSION_HISTORY.md ("PARITY ROADMAP"): gfx, meta (.ymt/.ymap/.ytyp), map/radar, 3D viewer, mod manager, AI. The original Zyn-RPF-main.zip and Zyn.rpf stay OUTSIDE the repo; ask the user to attach them again.
