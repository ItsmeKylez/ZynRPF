#!/bin/sh
# Runs every check from M1-M5a on a machine with Rust, Node and Playwright (Chromium). Usage: sh scripts/run-all-checks.sh
# Needs: cargo, node, `npm ci` + `npm run build` done in web/ (or scripts/build-web-offline.sh), and `playwright` resolvable from node.
# Each suite gets a fresh server with the settings written in that suite's header. Prints one PASS/FAIL line per suite, then a summary.
set -u
cd "$(dirname "$0")/.."
T="${TMPDIR:-/tmp}/rpf-checks"; rm -rf "$T"; mkdir -p "$T"
PORT=18080; RESULTS=""; FAILED=0
cargo build --release --bins --examples -p server -p rpf-core || { echo "BUILD FAILED"; exit 1; }
[ -f web/dist/index.html ] || (cd web && npm ci && npm run build) || sh scripts/build-web-offline.sh || { echo "WEB BUILD FAILED"; exit 1; }
GEN="cargo run --release -q -p rpf-core --example gen --"
$GEN "$T/good.rpf" 5 6291456; $GEN "$T/big.rpf" 30000 1000; $GEN "$T/small.rpf" 5 50000
cargo test --release || { RESULTS="$RESULTS\nFAIL cargo test"; FAILED=1; }

serve() { # usage: serve <name> VAR=value ...
  name="$1"; shift; D="$T/data-$name"; mkdir -p "$D"
  env DATA_DIR="$D" PORT=$PORT WEB_DIR="$PWD/web/dist" "$@" ./target/release/server >"$T/server-$name.log" 2>&1 & SPID=$!
  i=0; while [ $i -lt 50 ]; do curl -fs "http://127.0.0.1:$PORT/" >/dev/null 2>&1 && break; i=$((i+1)); sleep 0.2; done
}
stop() { kill $SPID 2>/dev/null; wait $SPID 2>/dev/null; PORT=$((PORT+1)); }
run() { # usage: run <name> <command...>
  name="$1"; shift
  if "$@" >"$T/out-$name.log" 2>&1 && ! grep -q "^FAIL\|  FAIL" "$T/out-$name.log"; then RESULTS="$RESULTS\nPASS $name"; else RESULTS="$RESULTS\nFAIL $name (see $T/out-$name.log)"; FAILED=1; fi
}
BIG="CHUNK_SIZE_BYTES=1048576 RATE_READS_PER_MIN=2000 RATE_WRITES_PER_MIN=2000"

serve e2e CHUNK_SIZE_BYTES=1048576 SESSION_QUOTA_BYTES=20971520 RATE_READS_PER_MIN=200 RATE_WRITES_PER_MIN=100
run e2e node scripts/e2e.mjs $PORT "$T/data-e2e" "$T/good.rpf" "$T/big.rpf"; stop

serve e2e-m3 SESSION_QUOTA_BYTES=3000000 MAX_BLOB_BYTES=1048576 RATE_READS_PER_MIN=2000 RATE_WRITES_PER_MIN=2000 CHUNK_SIZE_BYTES=1048576
run e2e-m3 node scripts/e2e-m3.mjs $PORT "$T/data-e2e-m3" "$T/small.rpf"; stop

serve e2e-m4 GLOBAL_QUOTA_BYTES=120000 COOKIE_SECURE=true
run e2e-m4 node scripts/e2e-m4.mjs $PORT "$T/small.rpf"; stop

for s in ui-test-m3 ui-test-m5a ui-test-m5a-drop; do
  serve $s $BIG SESSION_QUOTA_BYTES=50000000
  run $s node scripts/$s.cjs $PORT "$T/good.rpf"; stop
done
$GEN "$T/compressed.rpf" compressed
serve e2e-m5b
run e2e-m5b node scripts/e2e-m5b.mjs $PORT "$T/compressed.rpf"; stop

serve e2e-export
run e2e-export node scripts/e2e-export.mjs $PORT "$T/compressed.rpf"; stop
serve ui-test-m5a-tabs $BIG SESSION_QUOTA_BYTES=50000000
run ui-test-m5a-tabs node scripts/ui-test-m5a-tabs.cjs $PORT "$T/good.rpf" "$T/compressed.rpf"; stop

serve e2e-nested SESSION_QUOTA_BYTES=3000000 MAX_BLOB_BYTES=1048576 RATE_READS_PER_MIN=2000 RATE_WRITES_PER_MIN=2000 CHUNK_SIZE_BYTES=1048576
run e2e-nested node scripts/e2e-nested.mjs $PORT "$T/data-e2e-nested" "$T/small.rpf"; stop
$GEN "$T/ypt.rpf" ypt
serve e2e-ypt SESSION_QUOTA_BYTES=50000000 RATE_READS_PER_MIN=2000 RATE_WRITES_PER_MIN=2000
run e2e-ypt node scripts/e2e-ypt.mjs $PORT "$T/ypt.rpf"; stop
serve ui-test-ypt $BIG SESSION_QUOTA_BYTES=50000000
run ui-test-ypt node scripts/ui-test-ypt.cjs $PORT "$T/ypt.rpf"; stop
run test-yptbulk node scripts/test-yptbulk.mjs
run test-vsbulk node scripts/test-vsbulk.mjs
$GEN "$T/vs.rpf" vs
serve e2e-vs SESSION_QUOTA_BYTES=50000000 RATE_READS_PER_MIN=2000 RATE_WRITES_PER_MIN=2000
run e2e-vs node scripts/e2e-vs.mjs $PORT "$T/vs.rpf"; stop
serve ui-test-vs $BIG SESSION_QUOTA_BYTES=50000000
run ui-test-vs node scripts/ui-test-vs.cjs $PORT "$T/vs.rpf"; stop
$GEN "$T/ytd.rpf" ytd
serve e2e-ytd SESSION_QUOTA_BYTES=50000000 RATE_READS_PER_MIN=2000 RATE_WRITES_PER_MIN=2000
run e2e-ytd node scripts/e2e-ytd.mjs $PORT "$T/ytd.rpf"; stop
serve ui-test-ytd $BIG SESSION_QUOTA_BYTES=50000000
run ui-test-ytd node scripts/ui-test-ytd.cjs $PORT "$T/ytd.rpf"; stop
run english-audit node scripts/audit-english.mjs

printf "$RESULTS\n"
[ $FAILED -eq 0 ] && echo "ALL PASSED" || echo "SOME FAILED: send me $T/out-*.log"
exit $FAILED

node scripts/test-rsc7.mjs
node scripts/test-textfile.mjs
node scripts/test-textcheck.mjs
node scripts/test-pending-grid.mjs
