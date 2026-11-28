# M2 Vertical Slice — Notes (IN PROGRESS; not a stage release)

Status as of M2-CP005 (2026-11-27): READY FOR REVIEW. Everything below was run and observed unless marked UNVERIFIED.

## What works (verified)
Browser-style upload → per-session server workspace → existing Rust core opens the RPF → archive tree as JSON → download of a single entry, of the untouched upload, and of the re-serialized archive (re-parsed before being offered).

Verified by: `cd web && cargo test --workspace` (24 tests), a headless-Chromium run of the dev harness page (upload via file input, tree, entry and export downloads, close; hostile entry name stays inert text), a release build (`cargo build --release -p zyn-server`, 1m40s) whose binary opened a synthetic archive, and a manual run of the real binary with `curl` (upload, open, entry, export, download, CSRF 403, empty workspace for a cookie-less client).

## Run it
```
cd web
cargo run -p zyn-server                      # PORT (default 8080), binds 0.0.0.0
PORT=8080 WORKSPACE_ROOT=/tmp/zyn COOKIE_SECURE=false cargo run -p zyn-server
cargo run -p zyn-server --example make_sample_rpf -- sample.rpf   # synthetic test archive
```
`web/.cargo/config.toml` adds `-lstdc++` (needed to link `intel_tex_2`'s prebuilt ISPC objects on Linux).

Environment: `PORT`, `WORKSPACE_ROOT`, `MAX_UPLOAD_BYTES`, `MAX_WORKSPACE_BYTES`, `MAX_WORKSPACES`, `MAX_FILES_PER_WORKSPACE`, `WORKSPACE_IDLE_TTL` (s), `WORKSPACE_MAX_AGE` (s), `COOKIE_SECURE` (true/false). An invalid value stops startup with an error.

## Routes implemented
All state-changing requests need `X-Requested-With: zyn`. Workspace comes only from the `zyn_session` cookie (HttpOnly, SameSite=Strict, Secure unless disabled). Errors: `{"error":{"code","message"}}`.

| Route | Notes |
|---|---|
| `GET /`, `GET /dev.js` | M2 dev harness page (not the app UI), strict CSP, no inline script |
| `GET /healthz`, `GET /readyz` | no session; readyz checks the workspace dir is writable |
| `GET /api/v1/session` | creates/returns session, limits |
| `DELETE /api/v1/session` | deletes workspace + open documents |
| `POST /api/v1/files` (multipart) | streamed, size-capped; returns `[{fileId,name,size}]` |
| `GET /api/v1/files`, `GET /api/v1/files/{fileId}/download` | list; streams the immutable upload |
| `POST /api/v1/documents {fileId}` | open (idempotent per file); max 8 open |
| `GET /api/v1/documents`, `GET /api/v1/documents/{id}`, `DELETE …/{id}` | list / summary+tree / close |
| `GET /api/v1/documents/{id}/entry?path=` | one file entry |
| `GET /api/v1/documents/{id}/export` | verified re-serialized archive (**M2 addition, not in the M1 contract**) |

## Deviations from `M1_API_CONTRACT.md`
- Upload response has no `sha256` / sniffed `kind` yet.
- `…/export` is new; contract's `save` / `files/{id}/download?version=current` need backups + a `current` file (not built).
- Malformed JSON bodies return axum's default plain-text rejection, not the JSON error shape.
- `If-Match`/revision checks, `DELETE /files/{id}`, jobs/SSE, previews: not implemented.

## Known limitations / UNVERIFIED
- Whole archive is read into memory on open; a 512 MiB upload limit could exhaust RAM on a small Railway plan (UNVERIFIED, not measured).
- Randomness is read from `/dev/urandom` (Unix only; fine for Railway/Linux).
- No SPA/static file serving for the real frontend (M3), no Dockerfile yet. Release build tested on Linux only; Docker image NOT tested.
- Only synthetic archives tested; no real-world `.rpf` (none supplied). Encrypted (AES/NG) archives only via core's error path, untested here.
- No rate limiting, no `ALLOWED_ORIGINS`/CORS handling, no request logging.
- Per-workspace data is ephemeral (Railway disk): users must download their work.
