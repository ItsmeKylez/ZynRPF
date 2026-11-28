# Web Conversion Baseline

## What is actually present in this source archive

The archive root is `Zyn-RPF-main/`. It is a Tauri 2 + React + TypeScript + Vite desktop application.

### Relevant paths
| Path | Observed purpose |
|---|---|
| `core/` | Rust library crate for format/archive processing |
| `core/src/rpf.rs` | RPF archive implementation |
| `core/src/rsc.rs` | RSC/resource implementation |
| `core/src/ytd.rs` | YTD/texture implementation |
| `core/src/ypt.rs` | YPT/particle implementation |
| `core/src/gfx.rs` | GFX implementation |
| `core/src/model.rs` | Model-related implementation |
| `core/src/harita.rs` | Map-related implementation |
| `core/src/doku.rs` | Texture-related implementation |
| `core/src/vs.rs` | Visual settings implementation |
| `core/src/modlar.rs` | Mod-related implementation |
| `core/src/oiv.rs` | OIV-related implementation |
| `src-tauri/src/lib.rs` | Tauri commands and desktop orchestration |
| `src-tauri/src/kasa.rs` | Credential storage |
| `src/api.ts` | Typed frontend API; imports Tauri `invoke` |
| `src/App.tsx` | Main application shell |
| `src/*.tsx` | Existing editor and UI components |
| `package.json` / `package-lock.json` | JS dependencies and scripts |
| `Cargo.toml` / `Cargo.lock` | Rust workspace and locked dependencies |

## Confirmed frontend component files

- `src/AiPanel.tsx`
- `src/App.tsx`
- `src/AyarEditoru.tsx`
- `src/Belge.tsx`
- `src/Cakisma.tsx`
- `src/DokuEditoru.tsx`
- `src/DosyaGorunumu.tsx`
- `src/Gfx.tsx`
- `src/HaritaEditoru.tsx`
- `src/HazirPanel.tsx`
- `src/Ozet.tsx`
- `src/Palet.tsx`
- `src/Uc.tsx`
- `src/Yedekler.tsx`
- `src/Ypt.tsx`
- `src/ai.ts`
- `src/api.ts`
- `src/hazirlar.ts`
- `src/main.tsx`
- `src/ortak.tsx`
- `src/tarama.ts`
- `src/yazi.ts`
- `src/yptToplu.ts`

## Confirmed Tauri coupling

`src/api.ts` imports `invoke` from `@tauri-apps/api/core` and wraps many named Tauri commands. The current UI therefore depends on a desktop IPC boundary. Do not replace this with scattered fetch calls inside components. Create a typed web API adapter that preserves a stable frontend-facing contract where possible.

`src-tauri/src/lib.rs` contains the Tauri command implementation and substantial application orchestration. Inspect command annotations and function bodies to map each command to the existing Rust core. Do not assume all functions are portable as-is: some rely on local paths, OS dialogs, credential storage, or desktop state.

## Existing command/function discovery

A source scan found these function names in `src-tauri/src/lib.rs` (some are helpers, not necessarily exposed Tauri commands):

`mods_tara`, `mod_ac_kapa`, `arsiv_ac`, `yeni_mod`, `belge_ozet`, `belge_kapat`, `geri_al`, `yinele`, `kaydet`, `mod_sikistir`, `yedekleri_listele`, `yedek_geri_yukle`, `moda_kur`, `icerikte_ara`, `dosya_oku`, `metin_yaz`, `ice_aktar`, `dosya_degistir`, `disa_aktar`, `sil`, `yeniden_adlandir`, `dizin_olustur`, `tasi`, `ytd_listesi`, `doku_degistir`, `doku_renklendir`, `ytd_islem`, `gfx_ac`, `gfx_bilgi`, `gfx_degistir`, `gfx_disa`, `gfx_png`, `ypt_ayarlari`, `ypt_yaz`, `meta_yaz`, `doku_adlandir`, `ai_http`, `ayarlar_oku`, `ayarlar_yaz`, `doku_disa`, `doku_toplu_disa`, `doku_toplu_ice`, `vs_oku`, `vs_ayarla`, `model_ozetleri`, `model_dokulari`, `harita_duzeni`, `harita_isle`, `model_obj`, `harita_verisi`, `doku_png`, plus helpers.

This is a discovery list, not a complete API specification. Verify exact Tauri command attributes, parameter names, return types, and core dependencies in source before porting.

## Existing README claims / scope hints

The README describes support for:
- OPEN and CFXP RPF archives; NG/AES encrypted archives are listed as unsupported.
- Texture editing and embedded textures in `.ydr`, `.ydd`, `.yft`, `.ypt`.
- Assembly mapping and mod conflict analysis.
- `visualsettings.dat` editing.
- `.ypt` particle editing.
- 3D model and collision viewing, OBJ export.
- Pause map/radar tile editing.
- `.gfx` embedded image inspection/replacement/export.
- `.ymt`, `.ymap`, `.ytyp` meta/XML editing.
- Optional AI assistant.
- Backups, undo/redo, search, presets and mod workflows.

Treat these as stated project scope only. Verify each against the implementation and tests before marking a checklist item complete.

## Important conversion risks to investigate

1. Local absolute paths are deeply embedded in desktop workflows. Replace with opaque workspace IDs and validated relative paths at the web boundary.
2. The frontend API wrapper is broad. Preserve its type safety and avoid a one-off API implementation per component.
3. The Rust core may be reusable, but some orchestration currently lives in the Tauri host. Extract reusable logic carefully without moving UI/OS-specific concerns into `core/`.
4. Windows credential storage in `kasa.rs` cannot be used as the web secret store. Keep provider keys server-side and never return them to frontend clients.
5. Map/editor image handling may currently use local files or custom Tauri protocol URLs. Replace with authenticated workspace asset endpoints or safe blob URLs.
6. Large RPF files require streaming or bounded upload handling, clear size limits, progress feedback, and cleanup.
7. Railway filesystem persistence must not be assumed. Explain temporary workspace lifetime and make exports downloadable.
8. The original README describes Windows-only desktop behavior. The web app must clearly explain that it cannot silently install into the user's local FiveM mods directory.

## Source-of-truth policy

When this baseline conflicts with actual code, actual code wins. Update this file with evidence and explain the discrepancy in `PROJECT_STATUS.md`.


## M0 audit results

The evidence-based audit (command table, protocol endpoints, desktop coupling, risks, baseline record) is in `docs/M0_SOURCE_AUDIT.md`. Anything in this baseline file that conflicts with it should be treated as superseded after user review.
