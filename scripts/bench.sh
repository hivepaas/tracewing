#!/usr/bin/env bash
# Runs the origin and the proxy as separate processes, sends load through the
# proxy, and reports throughput, latency, the proxy's peak memory and the
# stored flows.
#
#   N=1000000 C=64 SIZE=1024 HTTP1=1 ENC=gzip SETTINGS='index_bodies = false' scripts/bench.sh
set -euo pipefail
cd "$(dirname "$0")/.."

N=${N:-100000}
C=${C:-64}
SIZE=${SIZE:-1024}
HTTP1=${HTTP1:-}
PATH_QUERY=${RICH:+&rich=1}${ENC:+&enc=$ENC}
SETTINGS=${SETTINGS:-}

cargo build --release -q -p tracewing-dev
bin=target/release/tracewing-dev
work=$(mktemp -d)

"$bin" origin --listen 127.0.0.1:18443 --ca-out "$work/origin-ca.pem" 2>/dev/null &
origin=$!
trap 'kill $origin ${proxy:-} 2>/dev/null || true; rm -rf "$work"' EXIT
until [ -s "$work/origin-ca.pem" ]; do sleep 0.05; done

printf '%b\n' "$SETTINGS" >"$work/settings.toml"
"$bin" proxy --listen 127.0.0.1:19090 --data-dir "$work/ca" --session "$work/session" \
    --extra-root "$work/origin-ca.pem" --settings "$work/settings.toml" \
    2>"$work/proxy.log" &
proxy=$!
until [ -s "$work/ca/ca.pem" ]; do sleep 0.05; done

version=HTTP/2
[ -n "$HTTP1" ] && version=HTTP/1.1
url="https://localhost:18443/json?size=$SIZE$PATH_QUERY"
# The origin makes its pool on the first request; that is not what is timed.
"$bin" load --proxy http://127.0.0.1:19090 --url "$url" --proxy-ca "$work/ca/ca.pem" -n 1 -c 1 >/dev/null
echo "== $N requests, concurrency $C, body $SIZE B ${ENC:-plain}, client→proxy $version, settings: ${SETTINGS:-defaults}"
"$bin" load --proxy http://127.0.0.1:19090 --url "$url" \
    --proxy-ca "$work/ca/ca.pem" -n "$N" -c "$C" ${HTTP1:+--http1} --watch-pid "$proxy"

kill -TERM "$proxy"
wait "$proxy" || true
proxy=
grep -E "ProxySummary" "$work/proxy.log" | tail -1

python3 - "$work/session" <<'EOF'
import os, sqlite3, sys
session = sys.argv[1]
db = sqlite3.connect(os.path.join(session, "flows.db"))
rows, ok, resp = db.execute(
    "SELECT count(*), sum(status = 200), sum(resp_bytes) FROM flows").fetchone()
def size(*parts):
    path = os.path.join(session, *parts)
    if os.path.isfile(path):
        return os.path.getsize(path)
    return sum(os.path.getsize(os.path.join(path, f)) for f in os.listdir(path))
flows = sum(size(f) for f in ("flows.db", "flows.db-wal") if os.path.exists(os.path.join(session, f)))
parts = (flows, size("bodies"), size("files"), size("search"))
print(f"stored flows: {rows} ({ok} with 200), {(resp or 0) / 1e6:.0f} MB of responses")
print("on disk: rows {:.0f} MB, log {:.0f} MB, files {:.0f} MB, index {:.0f} MB, total {:.0f} MB".format(
    *(p / 1e6 for p in parts), sum(parts) / 1e6))
EOF
