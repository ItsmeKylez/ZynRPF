# Deploying RPF Editor on Railway (you deploy; nothing here was run on Railway)
Status: SUGGESTED steps. The Dockerfile and `railway.json` have NOT been built or run by the assistant (no Docker daemon or network was available). The entrypoint, startup cleanup, health checks and global storage cap were tested locally without Docker.

## Read this first: what this build is and is not
This is the M1-M3 build with the temporary deviations of D-039 still in place. Use it as a private or invite-only test, not as a public service:
- **No sandbox.** Parsing and rebuilding run inside the server process (D-024 is not implemented). A hostile archive that finds a bug in the parser can affect the whole server. This is the main blocker for public launch.
- **State is in memory.** Sessions, workspaces and pending edits are lost on every restart or redeploy, and the server wipes the volume at startup. Do not run more than one instance (replica).
- **Anonymous sessions only**, no accounts, no Postgres, no volume backups (spec defaults).
- **npm licences** are not yet checked by a gate (D-042).
- Real GTA V archive compatibility is not verified (acceptance N).

## Steps
1. Push this folder to a GitHub repository, then in Railway choose New Project, Deploy from GitHub repo. Railway reads `railway.json` and builds the `Dockerfile`.
2. Add a **Volume** to the service and mount it at `/data`. Pick a size (the guide below assumes 10 GB).
3. Set the variables below (Service, Variables).
4. Under Settings, Networking, generate a domain. Railway terminates HTTPS, so the secure cookie works.
5. Keep the service at **one replica**.
6. After the deploy is healthy, open the domain, upload a small archive, rename a file, Save, and download the new version.

## Variables
| Variable | Suggested value | Meaning |
|---|---|---|
| `PORT` | set by Railway | listen port |
| `DATA_DIR` | `/data` (image default) | the volume mount path |
| `COOKIE_SECURE` | `true` (image default) | Secure cookies; needs HTTPS |
| `GLOBAL_QUOTA_BYTES` | about 80% of the volume, for example `8000000000` | total storage across all sessions; new uploads get 503 `server_full` beyond it. **Set this.** Default 0 means off |
| `SESSION_QUOTA_BYTES` | `2147483648` (default) | per-session storage, all versions and pending files |
| `MAX_UPLOAD_BYTES` | `1073741824` (default) | largest archive |
| `MAX_BLOB_BYTES` | `268435456` (default) | largest single added or replacement file |
| `MAX_SESSIONS` | `200` while unsandboxed (default 10000) | caps simultaneous anonymous sessions |
| `IDLE_SECONDS` | `86400` (default) | delete a session's data after this much inactivity |
| `RATE_READS_PER_MIN`, `RATE_WRITES_PER_MIN` | `600`, `120` (defaults) | per-session rate limits |
| `MAX_CONNECTIONS` | `256` (default) | concurrent connections |
| `MAX_ENTRIES`, `MAX_WORKSPACES`, `CHUNK_SIZE_BYTES`, `UPLOAD_TTL_SECONDS` | defaults | see `crates/server/src/main.rs` |
All numeric values are starting points (D-029), not measured on Railway.

## Memory and size guide (SUGGESTED)
Measured locally: an archive with 1,000,000 entries parsed in about 0.1 s; replay of edits took about 0.3 to 0.4 s and a dry run about 0.5 s; server peak memory was about 285 MiB during those steps. A full save of that archive was not timed (the run was interrupted). Start with at least 1 GB of memory and watch it.

## Health checks
`GET /api/v1/health/live` (process is up) and `GET /api/v1/health/ready` (the data directory is writable). `railway.json` uses the ready check.

## Run the same image locally
```
docker build -t rpf-editor .
docker run --rm -p 8080:8080 -v rpfdata:/data -e GLOBAL_QUOTA_BYTES=2000000000 rpf-editor
```
The container starts as root only to clear stale data and fix volume ownership, then drops to the unprivileged `app` user.

## Before any public launch (from D-039, ROADMAP)
Sandboxed worker per job (D-024); persistent metadata (D-026 or a database) if restarts must not lose work; accounts or CAPTCHA decision (D-015); npm licence gate (D-042); real-archive testing and an in-game check (D-035); load test; backups decision.
