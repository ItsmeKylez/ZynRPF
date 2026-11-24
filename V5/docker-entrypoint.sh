#!/bin/sh
# Starts the server as an unprivileged user. A mounted volume (for example on Railway) is usually owned by root,
# so the container starts as root, makes the data directory writable for the app user, then drops privileges.
set -eu
DATA="${DATA_DIR:-/data}"
RUN_AS="${RUN_AS:-app}"
mkdir -p "$DATA"
if [ "$(id -u)" = "0" ]; then
  # Session data does not survive a restart (D-039), so anything left on the volume is stale. Clear it as root so
  # files owned by an earlier user cannot block the unprivileged server from starting cleanly.
  find "$DATA" -mindepth 1 -maxdepth 1 -exec rm -rf {} + 2>/dev/null || true
  chown "$RUN_AS" "$DATA" 2>/dev/null || true
  GID="$(id -g "$RUN_AS")"
  exec setpriv --reuid="$RUN_AS" --regid="$GID" --init-groups "$@"
fi
exec "$@"
