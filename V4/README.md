# RPF Editor (M1 backend + M2 web UI)

Clean-room, MIT-licensed, English-only web editor for GTA V / FiveM RPF archives. M2 adds a read-only web UI (browse, search, inspect, hex view, download). Editing and saving arrive in M3.

## Run (Linux, WSL or Docker; the server reads /dev/urandom)

    (cd web && npm ci && npm run build)       # builds web/dist
    cargo build --release
    PORT=8080 DATA_DIR=./data WEB_DIR=./web/dist target/release/server
    # then open http://127.0.0.1:8080

For UI development: `cd web && npm run dev` (proxies /api to port 8080).

## Tests

    cargo test --release                      # rpf-core: hostile fixtures, fuzz
    cargo run --release -p rpf-core --example gen -- /tmp/good.rpf 5 6291456
    cargo run --release -p rpf-core --example gen -- /tmp/big.rpf 20000 2000000
    cargo run --release -p rpf-core --example gen -- /tmp/mil.rpf 1000000 500
    head -c 100 /tmp/good.rpf > /tmp/trunc.rpf
    PORT=8098 DATA_DIR=/tmp/d CHUNK_SIZE_BYTES=1048576 SESSION_QUOTA_BYTES=20971520 RATE_READS_PER_MIN=200 RATE_WRITES_PER_MIN=100 target/release/server &
    node scripts/e2e.mjs 8098 /tmp/d /tmp/good.rpf /tmp/big.rpf      # M1 API checks (70)
    # M2 browser tests need Playwright (Chromium) and axe-core; AXE_PATH points to axe.min.js
    PORT=8099 DATA_DIR=/tmp/d2 WEB_DIR=web/dist CHUNK_SIZE_BYTES=1048576 target/release/server &
    AXE_PATH=/path/to/axe.min.js node scripts/ui-test.cjs 8099 /tmp/d2 /tmp/good.rpf /tmp/big.rpf /tmp/trunc.rpf /tmp/mil.rpf
    node scripts/audit-english.mjs            # English audit of the web UI

Configuration (environment): PORT, DATA_DIR, WEB_DIR, COOKIE_SECURE, MAX_UPLOAD_BYTES, SESSION_QUOTA_BYTES, CHUNK_SIZE_BYTES,
IDLE_SECONDS, UPLOAD_TTL_SECONDS, MAX_ENTRIES, MAX_WORKSPACES, MAX_SESSIONS, MAX_CONNECTIONS, RATE_READS_PER_MIN (default 600), RATE_WRITES_PER_MIN.
