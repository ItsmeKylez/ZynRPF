# GPL notice (read this before distributing)

Decision (user, 2026-11-25): this project reuses the original Zyn-RPF / tlk-rpf core by Talkdedsec
(https://github.com/Talkdedsec/tlk-rpf, GPL-3.0-only, Copyright (c) 2026 Talkdedsec) to match its behavior exactly.

Files copied VERBATIM from the original (Turkish identifiers kept; header comment added; only lines marked "WEB ADAPTATION" differ):
- crates/rpf-core/src/rsc.rs (core/src/rsc.rs; adaptation: MAX_DECODED size guard and `.take()` limit in `Kaynak::coz`, a decompression-bomb guard for untrusted uploads)
- crates/rpf-core/src/ypt.rs (core/src/ypt.rs, unchanged)
- crates/rpf-core/src/ytd.rs and doku.rs (core/src/ytd.rs, core/src/doku.rs, unchanged apart from a header comment; texture dictionaries, DDS, BC1/3/5/7 codecs)
- crates/rpf-core/src/vs.rs (core/src/vs.rs, unchanged; visualsettings.dat parser)
- crates/rpf-core/src/lib.rs, bottom block: `Hata`, `Sonuc`, `bozuk`, `u16/u32/u64_oku` (core/src/lib.rs, unchanged)
The server maps the Turkish names and messages to English JSON and English errors in crates/server/src/api/ypt.rs, api/vs.rs and api/textures.rs.

Files derived from the original (translated to English, adapted):
- web/src/yptbulk.ts and the layout/behavior of web/src/YptEditor.tsx (from src/yptToplu.ts and src/Ypt.tsx)
- web/src/TextureEditor.tsx (layout/behavior from the original texture view)
- web/src/vsbulk.ts and the layout/behavior of web/src/VsEditor.tsx (from src/hazirlar.ts and src/AyarEditoru.tsx; presets, tint and slider ranges)

Every further file derived from the original must be listed here and carry a header comment saying so.

Consequence: a distributed build that contains these files must be offered under GPL-3.0 (full text: LICENSE-GPL-3.0.txt),
with source. The older MIT `LICENSE` covers only the earlier clean-room code and no longer describes the whole program.
Keep the original author's copyright notice. Third-party notices of the original: its THIRD_PARTY_NOTICES.md.
