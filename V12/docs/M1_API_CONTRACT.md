# M1 — Proposed Web API Contract (design only)

Status: PROPOSAL for review; nothing implemented. Depends on decisions D1–D5 in `M1_ARCHITECTURE.md` (this file assumes D2 = English DTOs). Final shapes are frozen only after M2 proves the slice.

## 0. Conventions

- Base path `/api/v1`. JSON UTF-8 unless noted. Workspace is implicit (session cookie); there is **no** workspace id in URLs and no client-supplied server path anywhere.
- `fileId`: server-generated id of an uploaded file. `docId`: per-workspace integer of an open document. `path`: archive-internal virtual path, `/`-separated, passed as **query/body field**, never as a URL path segment (avoids encoding/traversal ambiguity).
- Mutating requests: header `X-Requested-With: zyn`; document-mutating requests may send `If-Match: <revision>` (revision = today's `durum_no`). Stale → `412`.
- Mutations return the updated `DocumentSummary` (equivalent of `BelgeOzet`) unless stated.
- Errors: `{ "error": { "code": "RPF_ENCRYPTED", "message": "…", "details"?: {} } }`. HTTP: 400 validation, 404 unknown/foreign id, 409 conflict, 412 stale revision, 413 too large, 415 unsupported type, 422 well-formed but unprocessable (bad/encrypted RPF), 429 rate limit, 500 internal (sanitized).
- Long operations return `202 {jobId}` (see §9).

### Field renames (D2)
`id→docId`, `agac→tree`, `dugum/cocuklar→node/children`, `tur→kind`, `boyut→size`, `kayitli→storedSize`, `degisti→modified`, `kirli→dirty`, `geri/ileri→undoDepth/redoDepth`, `durum_no→revision`, `sifreleme→encryption`, `paket→package`, `hedefler→targets`, `eksik_kaynak→missingSources`, `disk_boyutu→diskSize`, `tahmini_boyut→estimatedSize`, `doku→texture`, `genislik/yukseklik→width/height`, `bicim→format`, `seviye→mipLevels`, `destekli→supported`, `sigdirma→fit` (`stretch|fit|fill` ← `esnet|sigdir|doldur`), `renk→colorAdjust` (`hue,saturation,brightness,contrast,tint,tintStrength`). Binary-format names and `core` types are unchanged.

## 1. Session / health

| Method & route | Purpose | Replaces |
|---|---|---|
| `GET /healthz`, `GET /readyz` | liveness / storage writable | — |
| `GET /api/v1/session` | creates/returns session; `{limits, features:{ai:boolean,…}, expiresAt}` | — |
| `DELETE /api/v1/session` | delete workspace + all data now | — |

## 2. Files (uploads) and downloads

| Route | Purpose | Replaces |
|---|---|---|
| `POST /files` (multipart, 1..n parts; optional `relativePath` per part for folder uploads) | stream-upload; returns `[{fileId,name,size,sha256,kind}]` (`kind` sniffed: `rpf`,`image`,`other`) | all `open()` dialogs; `ice_aktar`/`dosya_degistir`/… sources |
| `GET /files` | list workspace files | `mods_tara` listing |
| `GET /files/{fileId}` | metadata | — |
| `GET /files/{fileId}/download?version=current\|original` | download | `kaydet` to disk / "save as" |
| `DELETE /files/{fileId}` | remove file (and its backups) | — |

## 3. Documents (archive lifecycle)

| Route | Body → Response | Replaces (class) |
|---|---|---|
| `POST /documents` `{fileId}` → `DocumentSummary` | open (idempotent per fileId) | `arsiv_ac` (IN) |
| `POST /documents` `{new:{name,author}}` | new empty mod archive | `yeni_mod` |
| `GET /documents` → `[{docId,name,packageName}]` | list open | `acik_belgeler` |
| `GET /documents/{docId}` | summary | `belge_ozet` |
| `DELETE /documents/{docId}` | close (client confirms dirty first) | `belge_kapat` |
| `POST /documents/{docId}/undo`, `/redo` | | `geri_al`, `yinele` |
| `POST /documents/{docId}/save` `{saveAsName?}` → `{summary,oldSize,newSize,backupId}` | serialize→verify→backup→atomic write | `kaydet` (FS/OUT) |
| `POST /files/{fileId}/compress` → `{oldSize,newSize,backupId}` | rewrite without dead space if smaller | `mod_sikistir` (FS) |
| `GET /documents/{docId}/search?q=` | path/texture/setting/text hits | `icerikte_ara` |
| `POST /documents/{docId}/copy-from` `{sourceDocId,sourcePath,targetPath}` | cross-doc copy (same workspace) | `belgeler_arasi_kopyala` |
| `POST /documents/{docId}/set-open-header` | OPEN encryption header | `open_yap` |

## 4. Archive tree editing

| Route | Request | Replaces |
|---|---|---|
| `GET /documents/{docId}/entries?path=` | `{kind:meta\|texture\|model\|resource\|setting\|text\|binary,size,text?,hexDump?,rscVersion?}` | `dosya_oku` |
| `PUT /documents/{docId}/entries/text` `{path,text}` | edit text | `metin_yaz` |
| `POST /documents/{docId}/entries/text` `{path,text}` | create text file | `metin_olustur` |
| `POST /documents/{docId}/import` multipart or `{dir, fileIds[]}` | add files/folders (nested `.rpf` auto-opened as today) | `ice_aktar` (IN) |
| `POST /documents/{docId}/replace` `{path,fileId}` | replace file/nested archive | `dosya_degistir` (IN) |
| `POST /documents/{docId}/export` `{paths[]}` → **ZIP stream** | extract | `disa_aktar` (OUT) |
| `GET /documents/{docId}/entries/raw?path=` | single-file download | (new; single-file case) |
| `POST /documents/{docId}/delete` `{paths[]}` | | `sil` |
| `POST /documents/{docId}/rename` `{path,newName}` | | `yeniden_adlandir` |
| `POST /documents/{docId}/mkdir` `{path}` | | `dizin_olustur` |
| `POST /documents/{docId}/move` `{paths[],dir}` | self-move guard retained | `tasi` |

## 5. Textures (ytd and embedded)

| Route | Request | Replaces |
|---|---|---|
| `GET /documents/{docId}/textures?path=` | `[TextureInfo]` | `ytd_listesi` |
| `POST /documents/{docId}/textures/replace` `{path,index,fileId,fit,colorAdjust}` | | `doku_degistir` |
| `POST /documents/{docId}/textures/recolor` `{path,indices[],colorAdjust}` | | `doku_renklendir` |
| `POST /documents/{docId}/textures/rename` `{path,index,name}` | | `doku_adlandir` |
| `POST /documents/{docId}/textures/ops` `{path,ops:[{op:delete\|put\|scale,…}]}` | `put` references `fileId`; single undo step; embedded-dictionary restrictions preserved | `ytd_islem` |
| `GET /documents/{docId}/textures/{index}/export?path=&format=png\|dds` | download | `doku_disa` |
| `POST /documents/{docId}/textures/export-all` `{path,format}` → ZIP | | `doku_toplu_disa` |
| `POST /documents/{docId}/textures/import-all` `{path,fit,fileIds[]}` → `{summary?,matched[],unmatched[]}` | match by filename stem | `doku_toplu_ice` (IN) |

## 6. Other editors

| Route | Replaces |
|---|---|
| `GET …/settings?path=` / `PUT …/settings` `{path,values:[[k,v]]}` (visualsettings) | `vs_oku` / `vs_ayarla` |
| `GET …/particles?path=` / `PUT …/particles` `{path,changes:[[idx,[f32;4]]]}` | `ypt_ayarlari` / `ypt_yaz` |
| `PUT …/meta` `{path,xml}` → `{summary,warnings[]}` (XML view comes from `entries`) | `meta_yaz` |
| `GET …/gfx?path=` → `{images[],external[]}`; `POST …/gfx/replace` `{path,character,fileId,fit}`; `GET …/gfx/{character}/export?path=` (PNG) | `gfx_bilgi`, `gfx_degistir`, `gfx_disa` |
| `GET …/models` (summaries); `GET …/models/textures?path=`; `GET …/models/obj?path=` → ZIP (obj+mtl+png) | `model_ozetleri`, `model_dokulari`, `model_obj` |
| `GET …/map/placements?path=` (ymap) | `harita_verisi` |
| `GET …/map/layout`; `POST …/map/bake` `{layers:[{fileId,center,size,rotation,opacity,blend}],variants[]}` → `202 {jobId}` | `harita_duzeni`, `harita_isle` (+`ilerleme`) |

(`…` = `/documents/{docId}`.)

## 7. Package / assembly.xml

| Route | Replaces |
|---|---|
| `PUT …/package` `{package}` | `paket_yaz` |
| `POST …/package/repair-source` `{old,new}` | `kaynak_onar` |
| `POST …/package/prune-missing` | `olu_satirlari_temizle` |
| `POST …/package/targets` `{source,archives[],path}` | `hedef_ekle` |
| `POST …/package/targets-with-file` `{fileId,archives[],path}` | `hedefli_ekle` (IN) |

## 8. Mods set, backups

| Route | Replaces |
|---|---|
| `GET /mods` → `ModReport` (core `modlar::tara` on server-owned dir) | `mods_tara` |
| `POST /mods/{fileId}/enabled` `{enabled}` | `mod_ac_kapa` |
| `POST /mods/conflicts` `{target,fileIds[]}` → `ConflictDetail` | `cakisma_incele` |
| `GET /backups`; `POST /backups/{backupId}/restore` `{fileId}`; `GET /backups/{backupId}/download` | `yedekleri_listele`, `yedek_geri_yukle` |
| *(none)* | `moda_kur`, `yedekler_klasoru`: **removed** (desktop-only; documented) |

## 9. Previews (replace `tlk://` protocol) — GET, workspace-scoped, `Cache-Control: private, max-age` keyed by `rev`

| Route | Query | Replaces |
|---|---|---|
| `/documents/{docId}/preview/texture` | `path,index,size,rev` (+ `hue,sat,bri,con,tint,power` for live colour preview) | `/doku`, `/onizle` |
| `/documents/{docId}/preview/model-texture` | `path,name,size,rev` | `/modeldoku` |
| `/documents/{docId}/preview/model` | `path,rev` (binary for three.js; `.ybn` collision) | `/model` |
| `/documents/{docId}/preview/gfx` | `path,character,size,rev` | `/gfx` |
| *(none)* | — | `/gorsel` removed (use `URL.createObjectURL`) |

## 10. Jobs
`GET /jobs/{jobId}` → `{state:queued|running|done|failed,done,total,result?,error?}`; `GET /jobs/{jobId}/events` (SSE) → same payload stream. Replaces Tauri event `ilerleme`. Job results/outputs are workspace-scoped and expire with it.

## 11. AI (server-side)
| Route | Purpose |
|---|---|
| `GET /ai/providers` | `[{id,name,kind,model,configured,vision}]` — **never** includes keys |
| `POST /ai/providers/{id}/key` `{key}` (only if D5 allows BYO) | hold key in server memory for the session |
| `GET /ai/providers/{id}/models` | replaces `modelleriGetir` / `ai_http GET /models` |
| `POST /ai/chat` `{providerId,messages,tools?,…}` (streamed) | replaces browser-side Anthropic SDK and `ai_http POST` |
Replaces `ayarlar_oku/yaz` for provider settings. Non-secret UI preferences stay in browser `localStorage` (already used).

## 12. Coverage check against the 60 commands
Covered or deliberately removed: arsiv_ac §3 · yeni_mod §3 · belge_ozet §3 · belge_kapat §3 · geri_al/yinele §3 · kaydet §3 · mod_sikistir §3 · yedekleri_listele/yedek_geri_yukle §8 · yedekler_klasoru **removed** · moda_kur **removed** · icerikte_ara §3 · dosya_oku/metin_yaz/metin_olustur §4 · ice_aktar/dosya_degistir/disa_aktar §4 · sil/yeniden_adlandir/dizin_olustur/tasi §4 · ytd_listesi/doku_degistir/doku_renklendir/doku_adlandir/ytd_islem §5 · doku_disa/doku_toplu_disa/doku_toplu_ice §5 · vs_oku/vs_ayarla/ypt_*/meta_yaz/gfx_* §6 · model_ozetleri/model_dokulari/model_obj §6 · harita_verisi/harita_duzeni/harita_isle §6 · gecici_gorsel_kaydet **removed** (upload instead) · paket_yaz/open_yap/kaynak_onar/olu_satirlari_temizle/hedef_ekle/hedefli_ekle §3/§7 · acik_belgeler/belgeler_arasi_kopyala §3 · mods_tara/mod_ac_kapa/cakisma_incele §8 · ayarlar_oku/ayarlar_yaz §11 (+localStorage) · ai_http §11 (**replaced**, not proxied).
Result: 57 mapped to routes, 3 removed with documented reason (`moda_kur`, `yedekler_klasoru`, `gecici_gorsel_kaydet`); `ai_http` and settings are redesigned rather than ported 1:1.
