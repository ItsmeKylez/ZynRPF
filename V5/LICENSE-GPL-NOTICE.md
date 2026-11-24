# GPL notice (read this before distributing)

Decision (user, 2026-11-25): this project reuses the original Zyn-RPF / tlk-rpf core by Talkdedsec
(https://github.com/Talkdedsec/tlk-rpf, GPL-3.0-only, Copyright (c) 2026 Talkdedsec) to match its behavior exactly.

Files derived from the original (translated to English, adapted to this crate's dependency-free code):
- crates/rpf-core/src/rsc.rs (from core/src/rsc.rs)
- crates/rpf-core/src/ypt.rs (from core/src/ypt.rs)
- web/src/yptbulk.ts and the layout/behavior of web/src/YptEditor.tsx (from src/yptToplu.ts and src/Ypt.tsx)

Every further file derived from the original must be listed here and carry a header comment saying so.

Consequence: a distributed build that contains these files must be offered under GPL-3.0 (full text: LICENSE-GPL-3.0.txt),
with source. The older MIT `LICENSE` covers only the earlier clean-room code and no longer describes the whole program.
Keep the original author's copyright notice. Third-party notices of the original: its THIRD_PARTY_NOTICES.md.
