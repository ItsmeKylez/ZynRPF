# M1 — Web Architecture (design only; no implementation)

Status: DESIGN FOR REVIEW. Based on `docs/M0_SOURCE_AUDIT.md`. Nothing here has been built or run. Items marked **DECISION** need the user's explicit answer before M2; each has a recommended default. Items marked **UNVERIFIED** rest on assumptions that M2 must test.

## 1. Goals and non-goals

Goals: upload RPF/mod files in a browser → server opens them with the existing Rust `core/` → browse, edit (all current editor families) → validate → download. Hosted on Railway from GitHub. English UI.

Non-goals (stated limitations, not silent drops): writing into a user's local FiveM folder (`moda_kur`, mods-folder scan of the real disk, Explorer reveal); OS credential vault; Windows file-lock detection. These get web-native equivalents (§6) or are documented as unavailable.

## 2. Component layout

```
core/            UNCHANGED. Byte-oriented format library (tlk-rpf-core).
src-tauri/       UNCHANGED in M1/M2. Desktop app stays buildable.
web/doc-engine/  NEW crate (Tauri-free). The document layer: Belge/Durum equivalents,
                 undo/redo, caches, tree summary, texture/model/map/package operations,
                 save+verify. Pure functions of (document bytes, request) → result. No fs paths, no HTTP.
web/server/      NEW crate. HTTP (routes, sessions, workspaces, uploads, downloads, jobs,
                 limits, AI proxy). Depends on core + doc-engine.
src/             Existing React app. Tauri imports replaced by an adapter layer (M3).
docs/            Architecture/API/deployment.
```

### DECISION D1 — where the document layer lives (affects desktop code)
| Option | What | Pros | Cons |
|---|---|---|---|
| **A (recommended)** | New `web/doc-engine` copies/adapts the Tauri-free logic out of `src-tauri/src/lib.rs`. Desktop untouched. | Honors "don't destroy desktop source"; M2 can't break the desktop app. | Two copies of ~1,500 lines can drift. Mitigation: record source line ranges in `doc-engine` header; later, optional follow-up milestone makes `src-tauri` depend on `doc-engine` (separate approval). |
| B | Extract the logic into a shared crate now and make `src-tauri` call it. | Single source of truth. | Edits desktop code before the web slice is proven; cannot be verified here (no Windows/Tauri build, no deps). |
| C | `web/server` depends on `src-tauri` crate. | none practical | Pulls `tauri` and WebView deps into the server. Rejected. |

### DECISION D2 — API field naming
Current IPC uses Turkish keys (`agac`, `doku`, `kirli`…). Options: (1) keep Turkish keys on the wire; (2) **recommended**: the server serializes **English DTOs** (`tree`, `dirty`, `textures`…) via `serde(rename)`, and the frontend adapter is the only place that knows both. Binary-format identifiers and `core` public types stay unchanged (rule 5). Cost: M3 must update TS types; benefit: English API and docs without touching `core`.
The exact English names are fixed in `docs/M1_API_CONTRACT.md` (proposed).

### DECISION D3 — HTTP stack and new dependencies
`Cargo.lock` already contains `tokio 1.53.1`, `hyper 1.11.1`, `tower 0.5.3` (transitive via Tauri/reqwest). It does **not** contain `axum`, `tower-http`, or a ZIP crate. Recommended: `axum` + `tokio` + `tower-http` (static files, limits, compression) + `zip` (streaming export) + `uuid`/`rand` (ids) + `tracing`. **Every one is a new dependency and requires your approval at M2 (and network access to fetch).** Versions are not chosen here because they could not be checked offline.

## 3. Workspace and session model

- **Identity:** no accounts in v1. First request creates an anonymous **session** → opaque random 256-bit token in an `HttpOnly; Secure; SameSite=Strict` cookie. One session = one **workspace** (`WORKSPACE_ID` = random, never derived from client input). Token comparison constant-time; unknown token → new empty workspace (never reveals existence of others).
- **Storage layout** (under `WORKSPACE_ROOT`, default `/tmp/zyn-workspaces`, configurable):
  ```
  {ws}/files/{fileId}/original        # immutable upload, never modified
  {ws}/files/{fileId}/meta.json       # display name, size, sha256, upload time
  {ws}/files/{fileId}/current         # latest saved bytes (after Save)
  {ws}/backups/{fileId}/{ts}.rpf      # backup before every save/restore/compress
  {ws}/tmp/                           # upload staging, export staging
  ```
  Clients never see or send server paths; they use `fileId` (UUID/ULID) and **archive-internal virtual paths** (`a/b.rpf/c/d.ytd`, `/` separated, as in the current tree).
- **Documents:** an open document is an in-memory engine object bound to `(workspace, fileId)`. Document ids are **per-workspace** counters (replaces the global u32). At most one open document per fileId (replaces the canonical-path de-dupe in `arsiv_ac`).
- **Isolation rules:** every route resolves `ws` from the cookie only; any `fileId`/`docId` not in that workspace → `404` (same as non-existent). Cross-document copy (`belgeler_arasi_kopyala`) only within one workspace.
- **Lifecycle:** idle TTL (default 2 h, `WORKSPACE_IDLE_TTL`) and max age (default 12 h); janitor task deletes expired workspace dirs and drops documents. Hard disk quota per workspace and global. **Railway disk is ephemeral**: everything is temporary; UI must say "download your work" (also warn on `beforeunload` when dirty — replaces `onCloseRequested`). Optional persistent volume is out of scope for v1 (**DECISION D4** below).
- **Originals preserved:** uploads are never overwritten. "Save" writes `current` after (1) serializing, (2) re-parsing the result, (3) backing up the previous `current` (or `original`), (4) atomic write (temp file + `fsync` + `rename` — same pattern as core's `guvenli_yaz`, reusable). Download serves `current`.

### DECISION D4 — limits (proposed defaults, all env-configurable)
Max single upload 512 MiB; max workspace disk 2 GiB; max open documents per workspace 8; max concurrent workspaces 50; max undo steps 60 (existing `GECMIS_SINIRI`); backups kept 20 per file (existing `YEDEK_SAYISI`). Real-world RPF sizes and the Railway plan's RAM/disk are **UNVERIFIED** — please state your Railway plan/limits, or accept these defaults for M2 tuning.

## 4. Memory and concurrency (UNVERIFIED — M2 must measure)

Facts from source: whole archive held as `Arc<[u8]>`; undo snapshots clone `Arsiv` (file data `Arc`-shared; worst case not analysed); ytd cache cap 384 MiB per document (`YTD_ONBELLEK_SINIRI`).
Design: (a) per-workspace and global memory budget; cache cap lowered via env (`YTD_CACHE_BYTES`, default 128 MiB) and shared eviction; (b) one `Mutex` per document as today, never held across `await`; (c) CPU-heavy work (`model_ozetleri`, `harita_isle`, bulk texture ops, `Arsiv::yaz`) runs on a bounded blocking pool (`spawn_blocking` + semaphore) with per-request timeout; (d) requests that mutate a document carry the document **revision** (`durum_no` today) in `If-Match`; stale → `412` (implements "handle stale document state").

## 5. Long jobs and progress

`harita_isle` emitted Tauri event `ilerleme`. Web: mutating long operations return `202 {jobId}`; `GET /jobs/{id}` (poll) and `GET /jobs/{id}/events` (SSE) give `{done,total,state,result|error}`. Short operations stay synchronous. Cancellation not in v1.

## 6. Desktop → web equivalents

| Desktop behavior | Web design |
|---|---|
| Open dialog (`open`) for rpf/images/files/folders | `<input type=file>` / folder via `webkitdirectory` (fallback: multi-file or ZIP upload) → multipart upload creates `fileId`s; commands take `fileId` |
| Save dialog / write file | `Save` (server-side, §3) then **Download** (`Content-Disposition: attachment`) |
| Export folder (`disa_aktar`, `doku_toplu_disa`, `model_obj`) | Streaming ZIP download |
| Bulk import folder (`doku_toplu_ice`, `ice_aktar` dirs) | Multi-file/folder upload staged under `tmp/`, then command references staged ids |
| Drag-and-drop with OS paths | HTML5 DnD with `File` objects → upload → command; drop-target logic (`data-birak`) kept, payload changes |
| Mods folder scan/toggle/conflicts (`mods_tara`, `mod_ac_kapa`, `cakisma_incele`) | Workspace "mods set": uploaded RPFs form a virtual `mods/` and `mods-kapali/` (enabled flag in `meta.json`). `core::modlar` already takes `&Path` → server points it at a **server-owned** per-workspace directory (`{ws}/mods/`, `{ws}/mods-kapali/`) built from hard links/copies of uploads. Client never supplies `klasor`. |
| `moda_kur` install to FiveM | Removed. UI states install must be done manually after download. |
| Windows backup folder + `openPath` | Backups listed/restored/downloaded via API |
| Explorer reveal, `openPath` | Removed (documented) |
| `/gorsel` local image preview | `URL.createObjectURL(file)` in browser; **no server endpoint** |
| `gecici_gorsel_kaydet` | Removed: UI-rendered PNG is uploaded directly and referenced by `fileId` |
| Window close guard | `beforeunload` |
| Credential vault + plaintext settings | See §7.3 |

## 7. Security design

### 7.1 Paths and archives
- Virtual paths: reject empty segments, `.`/`..`, NUL, `\` (normalized to `/` server-side only for assembly.xml game paths where core already does), length caps; archive-internal names pass through existing `rpf::ad_duzelt` and additionally a server validator. No client string is ever joined to a real filesystem path; only server-generated ids are. ZIP export names sanitized (zip-slip), upload ZIP extraction (if added) validates entries and caps count/total size/ratio.
- Upload validation: size cap streamed (not buffered), extension allowlist per route (`rpf`, images `png/jpg/jpeg/webp/bmp/tga/dds`, resource/meta files, generic file for "add file to archive"), magic-number check for RPF7 (`0x52504637`) and image decode through `core::doku::gorsel_ac` (existing). Decompression-bomb guards on image dimensions (existing 8192 cap in `gomulu_boyutlandir`; add global cap) — **UNVERIFIED** that `gorsel_ac` limits decode size.
- Malformed/encrypted input: parser errors map to `422` with `code` (`RPF_ENCRYPTED`, `RPF_INVALID`, `UNSUPPORTED_FORMAT`…). Panics: workspace workers catch panics (`panic = "unwind"` is already set in `Cargo.toml`) and return `500` without killing the process.

### 7.2 HTTP hardening
CSRF: cookie `SameSite=Strict` + required custom header (`X-Requested-With: zyn`) on mutating routes + `Origin` check. Per-session and per-IP rate limits. Strict CSP (self + `blob:`/`data:` for images; **no** third-party connect), `X-Content-Type-Options`, `Referrer-Policy`. Errors: stable `{code,message}`; **no server paths, no stack traces**; messages from core are passed through only after path-scrubbing. Request IDs logged server-side.

### 7.3 AI assistant and secrets
Today: Anthropic SDK runs in the browser (`dangerouslyAllowBrowser`) with the user's key; other providers go through Rust `ai_http`; keys returned to the UI by `ayarlar_oku`.
Web design: all provider calls go through `POST /ai/{provider}/…` on the server. Two key sources (**DECISION D5**): (1) **recommended for v1**: operator-provided keys in Railway env vars (`AI_PROVIDERS_JSON` or per-provider vars), never sent to the browser, feature hidden if none configured; (2) optional "bring your own key": key sent once over HTTPS, held **in server memory only** for the session, never persisted or echoed (`GET` returns `{configured:true}`). No SSRF: provider base URLs come from a server allowlist (fixed `https://` hosts, no localhost/private IPs, DNS-rebinding-safe resolution check); custom/OpenAI-compatible endpoints disabled unless an operator whitelists them. AI tool calls execute against the caller's workspace documents through the same routes/validators (no separate privileged path). Request/response size and token budgets capped; opt-in network disclosure dialog retained. `ai.ts` model lists/ids are left as authored.

## 8. Deployment (Railway) design

- Single container, multi-stage Dockerfile: (1) Node stage builds `dist/` (`npm ci && npm run build`); (2) Rust stage builds `web/server` release; (3) slim runtime image with the binary + `dist/`. Server serves `/api/*` and static files with SPA fallback; binds `0.0.0.0:$PORT`; `GET /healthz` (liveness, no auth) and `GET /readyz` (workspace dir writable).
- Env: `PORT`, `WORKSPACE_ROOT`, `MAX_UPLOAD_BYTES`, `WORKSPACE_*` limits, `YTD_CACHE_BYTES`, `ALLOWED_ORIGINS`, `AI_*`, `RUST_LOG`. Secrets only via Railway variables.
- **UNVERIFIED:** native build of `intel_tex_2`/`ispc_rt` inside the Docker image (may need ISPC or may ship pre-generated code), `rage-formats` licence, Railway memory limits. M2's first task: reproduce a release build in a container; if ISPC is required, record the exact install step.

## 9. Testing strategy (for M2+)
Reuse all `core` tests unchanged. New: engine unit tests (open/undo/redo/save-verify against synthetic archives built with `Arsiv::yeni`), route tests (upload → open → tree → edit → save → download → reopen) with synthetic RPFs so CI does not need copyrighted mods; workspace isolation tests (A cannot read/modify B's `fileId`/`docId`); path-traversal and zip-slip tests; limit tests; AI-proxy SSRF tests. Real-file round-trips remain gated on user-supplied `testdata/*.rpf`.

## 10. M2 vertical slice (definition only — not started)
Browser upload of one RPF → `POST /documents` → engine opens with `Arsiv::oku` → `GET` tree (matches `BelgeOzet`/`DugumBilgi` semantics) → download extracted single file and re-serialized archive (verified by re-parse). Plus `/healthz`, workspace creation/expiry, size limit, isolation test. Out of M2: editors, AI, mods set, jobs.

## 11. Pending decisions summary
D1 doc-layer location (rec. A) · D2 English DTO layer (rec. yes) · D3 approve new crates and network fetch for M2 · D4 limits / Railway plan · D5 AI key policy (rec. operator env keys) · D6 keep upstream GPL-3.0-only for the web code (owner decision; not legal advice).
