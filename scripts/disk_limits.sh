#!/usr/bin/env bash
# What happens when the disk runs out or cannot keep up, in a container so that
# neither touches the machine. Needs rootful docker; builds nothing.
#
#   scripts/disk_limits.sh full     # a 300 MB disk, more traffic than fits
#   scripts/disk_limits.sh slow     # 100 and 20 MB/s, more traffic than that
set -euo pipefail
cd "$(dirname "$0")/.."
root=$PWD
image=ubuntu:26.04
device=/dev/nvme0n1

# Runs inside the container. $1 is the session's disk, then N, SIZE, ENC, settings.
inner=$(cat <<'INNER'
set -eu
disk=$1 n=$2 size=$3 enc=$4 settings=$5
bin=/app/target/release/tracewing-dev
work=$(mktemp -d)
"$bin" origin --listen 127.0.0.1:18443 --ca-out "$work/origin-ca.pem" 2>/dev/null &
origin=$!
until [ -s "$work/origin-ca.pem" ]; do sleep 0.05; done
printf '%b\n' "$settings" >"$work/settings.toml"
"$bin" proxy --listen 127.0.0.1:19090 --data-dir "$work/ca" --session "$disk/session" \
    --extra-root "$work/origin-ca.pem" --settings "$work/settings.toml" 2>"$work/proxy.log" &
proxy=$!
until [ -s "$work/ca/ca.pem" ]; do sleep 0.05; done
url="https://localhost:18443/json?size=$size&enc=$enc"
"$bin" load --proxy http://127.0.0.1:19090 --url "$url" --proxy-ca "$work/ca/ca.pem" -n 1 -c 1 >/dev/null
echo "== $n requests of $size B $enc, settings: ${settings:-defaults}"
"$bin" load --proxy http://127.0.0.1:19090 --url "$url" --proxy-ca "$work/ca/ca.pem" \
    -n "$n" -c 64 --http1 --watch-pid "$proxy"
stopping=$(date +%s.%N)
kill -TERM "$proxy"
while kill -0 "$proxy" 2>/dev/null; do sleep 0.05; done
wait "$proxy" && status=0 || status=$?
echo "proxy exit $status, $(awk "BEGIN { printf \"%.1f\", $(date +%s.%N) - $stopping }") s to stop"
grep -E "ProxySummary|Error" "$work/proxy.log" | tail -2
grep -E "WARN" "$work/proxy.log" | sed -E 's/^[^ ]+ +//' | cut -c1-120 | sort | uniq -c | sort -rn | head -4
df -k "$disk" | tail -1 | awk '{ printf "disk: %.0f MB used, %.0f MB free\n", $3 / 1024, $4 / 1024 }'
du -sm "$disk/session"/* 2>/dev/null | tr '\n' ' '; echo
kill "$origin" 2>/dev/null || true
INNER
)

# The app's own stdout/stderr need nothing from the image but libc.
run_full() {
    docker run --rm --tmpfs /data:size=300m -v "$root:/app:ro" "$image" \
        bash -c "$inner" -- /data "$@"
}

# Throttled writes need a host path on the throttled device. A memory limit keeps
# the page cache from hiding the limit for the first many GB, and running as the
# user leaves files that can be deleted afterwards.
run_slow() {
    local rate=$1
    shift
    rm -rf "$root/target/slowdisk"
    mkdir -p "$root/target/slowdisk"
    docker run --rm --user "$(id -u):$(id -g)" --memory 4g --device-write-bps "$device:$rate" \
        -v "$root:/app:ro" -v "$root/target/slowdisk:/data" "$image" \
        bash -c "$inner" -- /data "$@"
    rm -rf "$root/target/slowdisk"
}

case ${1:-} in
full)
    # Bodies stop at the reserve; rows go on, and traffic never notices.
    run_full 60000 30000 identity 'compress_stored = false\nindex_bodies = false\nqueue_budget_mib = 64\ndisk_reserve_mib = 50'
    # No reserve at all: the writes themselves fail.
    run_full 60000 30000 identity 'compress_stored = false\nindex_bodies = false\nqueue_budget_mib = 64\ndisk_reserve_mib = 0'
    # With the index on, and no reserve: it has to survive as well.
    run_full 60000 30000 identity 'compress_stored = false\nqueue_budget_mib = 64\ndisk_reserve_mib = 0'
    # A limit below the size of the disk: nothing ever goes wrong.
    run_full 60000 30000 identity 'compress_stored = false\nqueue_budget_mib = 64\nsession_max_mib = 128\ndisk_reserve_mib = 50'
    ;;
slow)
    rm -rf "$root/target/slowdisk"
    mkdir -p "$root/target/slowdisk"
    echo "== does the limit hold? 20 MB/s asked for"
    docker run --rm --user "$(id -u):$(id -g)" --device-write-bps "$device:20mb" -v "$root/target/slowdisk:/data" "$image" \
        dd if=/dev/zero of=/data/probe bs=1M count=100 oflag=direct 2>&1 | tail -1
    docker run --rm --user "$(id -u):$(id -g)" --memory 4g --device-write-bps "$device:20mb" -v "$root/target/slowdisk:/data" "$image" \
        dd if=/dev/zero of=/data/probe bs=1M count=100 conv=fsync 2>&1 | tail -1
    for rate in none 100mb 20mb; do
        echo
        echo "==== disk limit: $rate"
        if [ "$rate" = none ]; then
            run_slow 100gb 100000 30000 identity ''
            run_slow 100gb 100000 30000 gzip ''
        else
            run_slow "$rate" 100000 30000 identity ''
            run_slow "$rate" 100000 30000 gzip ''
        fi
    done
    ;;
*)
    echo "usage: $0 full|slow" >&2
    exit 2
    ;;
esac
