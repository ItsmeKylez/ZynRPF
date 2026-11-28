# M0 — Source Audit (Zyn-RPF / tlk-rpf → web conversion)

Status: evidence gathered by reading source. Nothing here was executed (see §8 Baseline). No application code was changed.
Audited files: `src-tauri/src/lib.rs` (2013 lines, read in full), `src-tauri/src/kasa.rs` (full), `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `src/api.ts` (full), `core/src/lib.rs`, `core/src/modlar.rs` (FS-dependent parts), `core/tests/gercek.rs` (header + test names), `.github/workflows/ci.yml` (head), manifests. Other `core/src/*.rs` modules and the individual React components were inspected by signature / grep only, NOT read line by line (marked below).

Note: the repo is named `tlk-rpf` internally (crate `tlk-rpf-core`, product `tlk-rpf`, upstream `Talkdedsec/tlk-rpf`, GPL-3.0-only). Internal identifiers are Turkish (`arsiv`, `doku`, `dosya`, ...).

## 1. Architecture as found

| Layer | Fact (evidence) |
|---|---|
| `core/` (`tlk-rpf-core`) | Pure Rust library. RPF API is **byte-oriented**: `Arsiv::oku(Arc<[u8]>)`, `Arsiv::yaz() -> Vec<u8>`, `Dosya::veri()`. Texture/model/ypt/gfx/meta/vs/oiv/harita modules take `&[u8]`. Reusable from a server as-is. |
| Path-based part of core | Only `core/src/modlar.rs`: `varsayilan_klasor` (reads `LOCALAPPDATA`), `tara`, `cakisma_incele`, `ac_kapa`, `kilit_kontrol`, `guvenli_yaz`, `yedekle`, `yedekleri_oku`. These take `&Path`, work on a FiveM `mods/` folder + sibling `mods-kapali/`. `kilit_kontrol` has a `#[cfg(windows)]` share-mode check. |
| `src-tauri/src/lib.rs` | **Not a thin wrapper.** Holds the whole document layer: `Belge` (archive + 60-step undo/redo snapshots, dirty tracking via `durum_no`/`temiz_no`, ytd cache capped at 384 MB, texture index cache, generation counter), `Durum` (global `HashMap<u32, Arc<Mutex<Belge>>>`, u32 counter), tree/summary building (`ozet`, `agac_kur`), model-texture lookup, map-layout, backup/save/refresh-open-docs logic, OBJ export, search. All of it depends on `tauri` types only at the command edges (`State`, `AppHandle`, `Uri`). |
| `src-tauri/src/kasa.rs` | API-key vault: Windows Credential Manager via `keyring` (`#[cfg(windows)]`); on non-Windows the functions are no-ops and **keys stay plaintext in `ayarlar.json`**. |
| Frontend `src/` | React 19 + TS + Vite. `api.ts` = 59 `invoke` wrappers (+1 direct `invoke("harita_verisi")` in `Uc.tsx:619`) = **60 commands, matches `generate_handler!` (60)**. |
| Custom protocol | `tlk://` (`http://tlk.localhost` on Windows) async URI scheme, **not a command**: `/doku`, `/onizle` (texture PNG, with colour params), `/modeldoku`, `/model` (binary model or `.ybn` collision), `/gfx`, `/gorsel`. Used by `<img>`/three.js loaders via `dokuUrl`, `gfxUrl`, `yerelGorselUrl`, `PROTOKOL` in `api.ts`. |
| Event | `ilerleme` `{bitti,toplam}` emitted by `harita_isle`, consumed in `ortak.tsx:101` via `listen`. |

## 2. Dependency classification used below

- **PURE** — operates on the in-memory document only; port to HTTP unchanged.
- **IN** — also takes a *local path* the user picked (image/source file) → must become an uploaded-file reference.
- **OUT** — writes to a *local path/folder* → must become a download (file or ZIP).
- **FS** — reads/writes the real FiveM mods folder or OS dirs → needs workspace redesign.
- **DESKTOP** — no web meaning.

## 3. All 60 commands

Args are the Rust names (Turkish). `id` = open-document id (u32). `Cevap<T>` = `Result<T,String>`; **all errors are plain Turkish strings** and some embed absolute local paths (e.g. `"{yol}: {e}"`).

### 3a. Document lifecycle / archive

| # | Command(args) → return | Core / logic | Class | Proposed web mapping (for M1 to confirm) |
|---|---|---|---|---|
| 1 | `arsiv_ac(yol)` → BelgeOzet | `fs::read` + `Arsiv::oku`; de-dupes by canonical path (`ayni_yol`, lowercases and swaps `/`→`\`) | IN/FS | `POST /workspaces/{w}/documents {fileId}` |
| 2 | `yeni_mod(ad,yazar)` → BelgeOzet | `Arsiv::yeni`, `oiv::uret` | PURE | `POST /workspaces/{w}/documents {new:{name,author}}` |
| 3 | `belge_ozet(id)` | `ozet()` | PURE | `GET …/documents/{id}` |
| 4 | `belge_kapat(id)` | remove from map | PURE | `DELETE …/documents/{id}` |
| 5 | `geri_al(id)` / 6 `yinele(id)` | pop/push snapshot stacks | PURE | `POST …/undo`, `…/redo` |
| 7 | `kaydet(id, hedef?)` → KayitSonucu | `Arsiv::yaz`, re-parse verify, `modlar::yedekle`, `modlar::guvenli_yaz`, `acik_belgeyi_tazele` | FS/OUT | Save = write into workspace file with backup + verify; user gets it via download endpoint. `hedef` ("save as") → new workspace file name |
| 8 | `mod_sikistir(yol)` → {eski,yeni,yedek} | read, rewrite, verify, backup, safe write (only if smaller) | FS | operate on workspace file id |
| 9 | `moda_kur(id, klasor)` → String | writes archive into FiveM mods dir | FS/DESKTOP | **No web equivalent** (cannot touch local FiveM). Replace with download; document limitation |
| 10 | `yedekleri_listele()` / 11 `yedek_geri_yukle(yedek,hedef)` / 12 `yedekler_klasoru()` | `modlar::yedekleri_oku`, `yedekle`, `guvenli_yaz`; backup dir = Tauri `app_data_dir()/yedek`; restore restricted to files whose parent is that dir; keeps 20 per source (`YEDEK_SAYISI`) | FS | per-workspace backup store; list/restore/download. #12 (returns OS path, frontend passes it to `openPath`) = DESKTOP |
| 13 | `icerikte_ara(id,sorgu)` | walk archive, texture-name index, text grep (≤400 results, skips >4 MB / binary) | PURE | `GET …/search?q=` |

### 3b. Archive tree editing

| # | Command | Class | Note |
|---|---|---|---|
| 14 | `dosya_oku(id,yol)` → DosyaIcerigi (kind/meta XML/text/hex dump) | PURE | |
| 15 | `metin_yaz(id,yol,metin)` | PURE | |
| 16 | `metin_olustur(id,yol,metin)` | PURE | |
| 17 | `ice_aktar(id,dizin,kaynaklar[])` | IN | `dugum_dosyadan` **recurses directories and auto-opens nested `.rpf`**; web: multipart upload (files/folders via `webkitdirectory`) |
| 18 | `dosya_degistir(id,yol,kaynak)` | IN | replaces file or nested archive from local path → upload |
| 19 | `disa_aktar(id,yollar[],hedef)` → count | OUT | `dugum_yaz` writes tree to disk (nested archives re-serialized) → stream ZIP |
| 20 | `sil(id,yollar[])`, 21 `yeniden_adlandir`, 22 `dizin_olustur`, 23 `tasi` | PURE | `tasi` guards moving a dir into itself |
| 24 | `belgeler_arasi_kopyala(kaynak_id,kaynak_yol,hedef_id,hedef_yol)`, 25 `acik_belgeler()` | PURE | cross-document: both docs must be in same workspace |

### 3c. Textures (ytd and embedded in ydr/ydd/yft/ypt)

| # | Command | Class | Note |
|---|---|---|---|
| 26 | `ytd_listesi(id,yol)` → DokuBilgi[] | PURE | `ytd_al` also handles model/ypt embedded dictionaries (`gomulu_dokulu`) |
| 27 | `doku_degistir(id,yol,i,gorsel,sigdirma,renk)` | IN | image path → upload ref |
| 28 | `doku_renklendir(id,yol,indeksler[],renk)` | PURE | |
| 29 | `doku_adlandir(id,yol,i,ad)` | PURE | |
| 30 | `ytd_islem(id,yol,islemler[])` (`sil`/`koy`/`olcekle`) | IN | `koy.gorsel` is a local path. For models/ypt: delete/add refused, only in-place resize (`gomulu_boyutlandir`) |
| 31 | `doku_disa(id,yol,i,hedef)` | OUT | `.dds` vs PNG chosen by `hedef` extension → download |
| 32 | `doku_toplu_disa(id,yol,klasor,dds)` | OUT | → ZIP |
| 33 | `doku_toplu_ice(id,yol,klasor,sigdirma)` | IN | reads a **folder** of images, matches by name → multi-file upload |

### 3d. Other editors

| # | Command | Class | Note |
|---|---|---|---|
| 34 | `vs_oku` / 35 `vs_ayarla(id,yol,degerler[])` | PURE | visualsettings.dat |
| 36 | `ypt_ayarlari(id,yol)` / 37 `ypt_yaz(id,yol,degisiklikler[(idx,[f32;4])])` | PURE | |
| 38 | `meta_yaz(id,yol,xml)` → {ozet,uyarilar} | PURE | re-reads result to verify before applying |
| 39 | `gfx_bilgi` / 40 `gfx_degistir(…gorsel)` (IN) / 41 `gfx_disa(…hedef)` (OUT) | | |
| 42 | `model_ozetleri(id)` (rayon parallel) / 43 `model_dokulari(id,yol)` | PURE | |
| 44 | `model_obj(id,yol,klasor)` → {obj,doku,eksik} | OUT | writes `.obj/.mtl/.png` to a folder → ZIP |
| 45 | `harita_verisi(id,yol)` (ymap placements) | PURE | **only command not in `api.ts`** (called raw in `Uc.tsx`) |
| 46 | `harita_duzeni(id)` | PURE | |
| 47 | `harita_isle(id,katmanlar[],varyantlar[])` | IN + event | layers reference local image paths; emits `ilerleme` progress → needs SSE/WebSocket/polling |
| 48 | `gecici_gorsel_kaydet(bayt)` → path | DESKTOP-artifact | saves UI-rendered PNG (text layer, AI) to cache dir and returns a path so another command can read it → in web, upload directly; remove the indirection |

### 3e. Package (assembly.xml / OIV)

| # | Command | Class |
|---|---|---|
| 49 | `paket_yaz(id,paket)` | PURE |
| 50 | `open_yap(id)` (sets header encryption to OPEN) | PURE |
| 51 | `kaynak_onar(id,eski,yeni)` | PURE |
| 52 | `olu_satirlari_temizle(id)` | PURE |
| 53 | `hedef_ekle(id,kaynak,arsivler[],yol)` | PURE |
| 54 | `hedefli_ekle(id,dosya{kaynak,arsivler,yol})` | IN (kaynak is a local path) |

### 3f. Mods folder

| # | Command | Class | Note |
|---|---|---|---|
| 55 | `mods_tara(klasor?)` → ModRaporu+`bulundu` | FS | default = `%LOCALAPPDATA%\FiveM\FiveM.app\mods`; error `"LOCALAPPDATA bulunamadı"` on non-Windows |
| 56 | `mod_ac_kapa(klasor,dosya,acik)` | FS | renames file between `mods/` and `mods-kapali/`; refuses names with `/`, `\`, `..` |
| 57 | `cakisma_incele(klasor,hedef,modlar[])` | FS | same validation of names; `Path::new(&klasor)` is client-supplied |

### 3g. Settings & AI

| # | Command | Class | Note |
|---|---|---|---|
| 58 | `ayarlar_oku()` / 59 `ayarlar_yaz(ayarlar)` | DESKTOP/FS | `app_config_dir()/ayarlar.json`; keys split to Windows vault (`kasa.rs`); **returns plaintext API keys to the webview** |
| 60 | `ai_http(yontem,url,basliklar,govde?)` | server-side HTTP proxy | `reqwest`, 600 s timeout, only `https://`, `http://localhost`, `http://127.0.0.1` allowed. **On a public server this is an open SSRF/relay** unless redesigned (allowlist providers, no localhost, key stored server-side) |

(Numbering above groups commands by function; all 60 names in `generate_handler!` are covered: mods_tara, mod_ac_kapa, arsiv_ac, yeni_mod, belge_ozet, belge_kapat, geri_al, yinele, kaydet, yedekler_klasoru, mod_sikistir, yedekleri_listele, yedek_geri_yukle, moda_kur, icerikte_ara, dosya_oku, metin_yaz, ice_aktar, dosya_degistir, disa_aktar, sil, yeniden_adlandir, dizin_olustur, tasi, ytd_listesi, doku_degistir, doku_renklendir, doku_adlandir, ytd_islem, meta_yaz, doku_disa, ayarlar_oku, ayarlar_yaz, ai_http, harita_verisi, model_obj, harita_duzeni, harita_isle, model_dokulari, model_ozetleri, cakisma_incele, doku_toplu_disa, doku_toplu_ice, ypt_ayarlari, gfx_bilgi, gfx_degistir, gfx_disa, ypt_yaz, gecici_gorsel_kaydet, metin_olustur, hedef_ekle, acik_belgeler, belgeler_arasi_kopyala, vs_oku, vs_ayarla, paket_yaz, olu_satirlari_temizle, kaynak_onar, open_yap, hedefli_ekle.)

Totals by class (approx., a command can be counted once under its strongest dependency): PURE ≈ 31, IN ≈ 9, OUT ≈ 6, FS ≈ 10, DESKTOP/settings/AI ≈ 4.

## 4. Protocol (`tlk://`) endpoints → HTTP GET

| Path | Query | Returns | Code | Web note |
|---|---|---|---|---|
| `/doku` | id,yol,i,boy,v | PNG of texture mip nearest `boy` | `doku_png` | cache by `v` (= `durum_no`); needs auth/workspace scoping |
| `/onizle` | + ton,doy,par,kon,guc,boya | PNG with colour adjust applied | `doku_png` | |
| `/modeldoku` | id,yol,ad,boy | PNG; resolves name in model first then archive ytds | `model_dokusu` | |
| `/model` | id,yol | binary model (`model::model_oku`) or collision (`.ybn` → `carpisma_oku`) | `model_paketi` | feeds three.js loader in `Uc.tsx` |
| `/gfx` | id,yol,c,boy | PNG | `gfx_png` | |
| `/gorsel` | yol (**arbitrary local path**) | PNG ≤2048px; allowed ext png/jpg/jpeg/webp/bmp/tga/dds | `yerel_gorsel` | **Must not be ported**: reads any local file. Replace with `blob:` URLs of the user's own upload |

## 5. Desktop coupling in the frontend (grep evidence)

- `@tauri-apps/api/core` `invoke`: `api.ts`, `Uc.tsx`. `listen`: `ortak.tsx`.
- `@tauri-apps/plugin-dialog` `open`/`save`: 29 call sites across `App, Belge, DokuEditoru, DosyaGorunumu, Gfx, HaritaEditoru, Ozet, Uc, Yedekler` (`open` for files/folders with extension filters; `save` for export names).
- `plugin-opener`: `openPath`, `revealItemInDir` (Explorer), `openUrl` (AI provider key links) in `App, Yedekler, AiPanel`. Explorer actions are meaningless on web.
- Window: `getCurrentWindow().onCloseRequested` (unsaved-changes guard, `App.tsx:189`) → `beforeunload`. `getCurrentWebview().onDragDropEvent` (`App.tsx:210`) gives **absolute OS paths** and window coordinates; web DnD gives `File` objects → whole drop handler (targets `doku:`, `dosya:`, `dizin:`, `ac`) needs rewriting.
- `PROTOKOL` chooses `http://tlk.localhost` by checking `userAgent` for Windows.
- `oyunYoluCoz` splits paths with `\` (Windows game paths in assembly.xml) — pure string logic, keep.
- CSP in `tauri.conf.json` allows `ipc:`, `tlk:` and `https://api.anthropic.com`.
- `localStorage` is used (settings keys, `tlk-rpf.sigdirma`) — works in browser.
- Turkish UI text: ~21 files contain Turkish characters (`ai.ts` ≈150 lines, `App.tsx` 56, `HaritaEditoru.tsx` 56, `Uc.tsx` 53, `DokuEditoru.tsx` 51, `Ypt.tsx` 43, `Ozet.tsx` 42, …). Count is lines containing ç/ğ/ı/ö/ş/ü only — **undercounts** strings with none of those letters.

Component → API usage (from grep `api.<name>`): `App` (open/save/undo/mods list/compress/new/drag-drop), `Belge` (tree ops, search, install, OPEN header), `DokuEditoru` (all texture ops), `Uc` (model + OBJ + map placement), `HaritaEditoru`, `Ozet` (package/model overview), `Gfx`, `Ypt`, `AyarEditoru`/`HazirPanel` (visual settings, presets), `Cakisma`, `Yedekler`, `DosyaGorunumu`, `AiPanel`, `ai.ts`, `tarama.ts`.

## 6. AI assistant (`src/ai.ts`, 1031 lines — grep-level only; NEEDS INVESTIGATION for full tool list)

- Two paths: Anthropic via `@anthropic-ai/sdk` **in the browser with `dangerouslyAllowBrowser: true` and the user's key** (`ai.ts:128,758`), and OpenAI-compatible providers via `api.aiHttp` (Rust proxy). Presets include Gemini and others (`HAZIR_SAGLAYICILAR`).
- Tools the model can call are implemented by calling `api.*` on open documents (read/write files, textures, ypt, map, visual settings, meta). `pngBase64(url)` fetches `tlk://` texture URLs to send images to the model.
- Web consequences: provider keys must never reach the browser; provider calls move server-side; tool execution must be scoped to the caller's workspace/document; `gecici_gorsel_kaydet` indirection disappears. Model IDs hard-coded in `ai.ts` (`claude-opus-5-5`, `claude-fable-5-1`, …) are the author's choice and were not changed.

## 7. Risks / open questions (carry into M1)

1. **Document layer lives inside `src-tauri/lib.rs`** and cannot be imported without pulling in Tauri. M1 must decide: extract it into a Tauri-free crate used by both desktop and web (touches desktop source; needs approval per rule 4) vs. duplicating it in a new `web-backend`. Duplicating risks drift; extracting is the lower-risk long-term path but changes the desktop crate.
2. **Memory model**: whole archives held in RAM (`Arc<[u8]>`), 60 undo snapshots per doc (`Arsiv` clones; data mostly `Arc`-shared — NEEDS INVESTIGATION for worst case), ytd cache up to 384 MB per document. Multi-user Railway container memory limits not yet analysed. Need per-workspace/document caps and eviction.
3. **Global u32 document ids** and path-keyed de-dupe/refresh (`acik_belgeyi_tazele`) assume a single user and a real filesystem. Needs workspace scoping and a virtual-path model.
4. **Windows path semantics**: `ayni_yol`/`yol_metni` normalise `\` and case; `yol_izi` backup key hashes the full OS path (FNV-1a) — backup identity must be redefined for workspace files.
5. **Open relay / local file read** risks: `ai_http` (SSRF) and `/gorsel` (arbitrary file read) must not be ported as-is. Error strings include local paths and must be sanitised.
6. **Secrets**: current non-Windows behaviour leaves AI keys plaintext in a config file and returns them to the UI. Not acceptable server-side.
7. **Native build risk on Railway (NEEDS INVESTIGATION)**: `Cargo.lock` shows `intel_tex_2` → `ispc_rt`; whether building needs an ISPC toolchain or ships pre-generated kernels was not verified. Also `texpresso` + `rayon`; `rage-formats 0.2.2` external crate (licence not checked). Edition 2024 requires a recent Rust (local `rustc 1.97.0`).
8. **Progress events** (`ilerleme`) need an HTTP equivalent (SSE/WebSocket/polling or job id).
9. **Long operations** (`harita_isle`, `model_ozetleri`, `doku_toplu_*`) run for seconds on a server thread pool; need timeouts/limits and request size limits for uploads (RPFs can be large; `core` tests mention >16 MB resources).
10. **Licence**: upstream is GPL-3.0-only. A web conversion is a derivative; keep licence/attribution and decide hosting/publishing terms. This is a project-owner decision, not legal advice.
11. **Real-file evidence gap**: `core/tests/gercek.rs` real-archive tests **skip** when `testdata/*.rpf` is absent (and it is absent from the archive). No round-trip evidence can be produced until the user supplies sample RPF files.
12. **Scope of "English"**: identifiers/JSON keys (`arsiv`, `doku`, `agac`, …) are part of the current IPC contract. M1 must decide whether the web API keeps Turkish field names internally (rule 5 permits keeping them) or exposes an English DTO layer. Not decided here.
13. **Encrypted archives**: core only edits OPEN/CFXP/none; others raise `Hata::Sifreli` ("only OPEN/CFXP archives can be edited"). Unchanged requirement: report honestly.

## 8. Baseline test/build record (honest)

| Command | Result | Evidence |
|---|---|---|
| `cargo --version`, `rustc --version` | `cargo 1.97.0`, `rustc 1.97.0` | ran |
| `cargo test -p tlk-rpf-core --offline` | **NOT RUN to completion — failed at dependency resolution** | `error: no matching package named 'bcdec_rs' found … crates.io index` (no vendored crates / registry cache in this environment; installs are disabled, so nothing was fetched) |
| `npm run build` / `npx tsc` | **NOT RUN** | no `node_modules/`; `node v22.23.5` present; installing is prohibited without approval |
| Tauri build | not run (not required) | |
| Static facts | `core/tests/gercek.rs` has 14 `#[test]`; `core/src` unit tests: harita 3, modlar 3, oiv 4, rpf 4, vs 1, ytd 1 | grep count of `#[test]` |
| CI reference | upstream CI runs on `windows-latest`: `npm ci`, `tsc --noEmit`, `vite build`, `cargo clippy --workspace --all-targets --locked -D warnings`, core tests | `.github/workflows/ci.yml` (head) |

No test or build is claimed to pass. To obtain a real baseline the user must approve dependency fetching (e.g. `cargo fetch`/`cargo test`, `npm ci`) or provide a pre-populated environment.

## 9. Answers to the START_HERE boundary questions

- *Rust code reusable directly by a web backend?* All of `core/` (byte-oriented), plus most of `lib.rs` logic **after extraction from Tauri types**.
- *Filesystem/dialog/window-specific commands needing web equivalents?* FS: #1,7–12,17–19,31–33,44,55–57 (see tables). Dialogs/opener/window: §5.
- *Components that can remain with only an adapter change?* Plausibly the editors that only call PURE commands and render results (`AyarEditoru`, `Ypt`, parts of `Cakisma`, `Gfx` display) — **not verified by reading each component**; every component that calls `open`/`save` (9 files) needs real changes.
- *APIs that pass local paths and must be redesigned around uploads?* Commands tagged IN/OUT/FS plus `/gorsel` and `gecici_gorsel_kaydet`.
- *Windows-only dependencies/facilities?* `LOCALAPPDATA` default mods path, `keyring` (Windows Credential Manager), share-mode lock check, `\` path normalisation, NSIS bundle target, `tlk.localhost` origin.
