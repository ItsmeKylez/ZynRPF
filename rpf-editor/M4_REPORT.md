# M4 REPORT: Deployment packaging (2026-11-24)
Scope as instructed by you: "Just do the code, I'll deploy it myself." So M4 here means deployment-ready code and instructions only. Nothing was deployed, and the Railway account was never touched.

## Added or changed
- `Dockerfile` (reworked), `docker-entrypoint.sh`, `.dockerignore`, `railway.json`, `DEPLOY_RAILWAY.md`.
- Server: `GLOBAL_QUOTA_BYTES` cap across all sessions (checked at upload start, content upload and save; 503 `server_full`). Startup now clears the *contents* of `DATA_DIR` instead of removing the directory (a mounted volume cannot be removed).
- Entrypoint: starts as root only to clear stale files and `chown` the volume, then drops to the `app` user with `setpriv`. This handles Railway volumes, which are root-owned.
- `scripts/e2e-m4.mjs`: health endpoints, Secure cookie, global cap.

## Verified here
- `scripts/e2e-m4.mjs`: 6 of 6 pass. Regressions after the changes: M3 API e2e 78 of 78, M1 API e2e 70 of 70.
- Entrypoint tested for real (without Docker): root-owned volume with stale root-owned files was cleared, the server ran as an unprivileged user, `/health/ready` returned 200, UI served. This found and fixed two bugs (group lookup for `setpriv`; stale root-owned files blocking startup).

## NOT verified
- **The Docker image has never been built.** No Docker daemon or network here. Check the first build log for npm or Rust problems (the web build is unverified with the pinned React 18.3.1 and Vite; I only bundled with a local esbuild and React 19).
- Railway behaviour (volume mount, healthcheck timing, domain/HTTPS, memory limits), load, and the full save time on a 1,000,000-entry archive (that run was interrupted).

## Still blocking a public launch (unchanged, D-039)
No sandbox for parsing/rebuilding, in-memory state (single instance, wiped on restart), no accounts/Postgres/backups, npm licence gate (D-042), no real-game verification. See `DEPLOY_RAILWAY.md`.

## Decisions (proposed): D-045
M4 = packaging only, deployed by you; `GLOBAL_QUOTA_BYTES` added (default off, must be set in production); container starts as root only to prepare the volume.
