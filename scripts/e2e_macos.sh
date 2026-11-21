#!/bin/sh
# Runs crates/e2e's tests, the ones that change the computer they run on —
# its proxy setting, its trust settings, its keychain, its firewall — on this
# Mac. Only on a Mac meant for it, or one whose owner says so: the first run
# needs --this-machine-may-be-changed, which makes the marker file the tests
# look for (with sudo); without the marker every test returns at once.
#
#   scripts/e2e_macos.sh --this-machine-may-be-changed
#   scripts/e2e_macos.sh --attended          # somebody here answers dialogs
#   scripts/e2e_macos.sh --skip-build
#   scripts/e2e_macos.sh --packages          # S4: the .dmg, built and installed
#
# --packages builds the disk image (scripts/package_macos.sh, which needs
# cargo-about) for the test that installs it into /Applications. A Mac that
# has a tracewing.app there already keeps it, and that test says so.
#
# macOS asks before tracewing trusts its CA (a user's password for their own
# trust settings, an administrator's for the machine's) and before it turns
# on the firewall rule that blocks UDP 443. On a CI runner nobody answers, so
# those tests say so and end, and the firewall's runs as root, which macOS
# does not ask. With --attended, somebody at this Mac answers each dialog as
# it comes, and every test runs as the user, as tracewing does.
#
# What a test changes it puts back, however it ends. On the keychain it takes
# out only what it put there: a Mac may be somebody's own, with tracewing's
# real key in it.
set -eu
cd "$(dirname "$0")/.."
target=${CARGO_TARGET_DIR:-target}
marker="/Library/Application Support/tracewing-e2e-throwaway"

attended=""
packages=""
build=yes
mark=""
for arg in "$@"; do
    case "$arg" in
    --this-machine-may-be-changed) mark=yes ;;
    --attended) attended=1 ;;
    --skip-build) build="" ;;
    --packages) packages=yes ;;
    *) echo "unknown: $arg" >&2; exit 2 ;;
    esac
done

if [ -n "$mark" ]; then
    sudo mkdir -p "$(dirname "$marker")"
    sudo touch "$marker"
elif [ ! -e "$marker" ]; then
    echo "Not marked as a machine these tests may change: pass --this-machine-may-be-changed." >&2
    exit 1
fi

if [ -n "$build" ]; then
    cargo build --release --locked -p tracewing
fi
tests=$(cargo test --release --locked -p tracewing-e2e --no-run --message-format=json \
    | python3 -I -c '
import json, sys
for line in sys.stdin:
    msg = json.loads(line)
    if msg.get("reason") == "compiler-artifact" and msg.get("executable") \
            and "test" in msg["target"]["kind"]:
        print(msg["executable"])
')

if [ -n "$packages" ]; then
    dist="$target/e2e-dist"
    rm -rf "$dist"
    BIN="$target/release/tracewing" DIST="$dist" scripts/package_macos.sh
    TRACEWING_E2E_DMG="$(pwd)/$(ls "$dist"/tracewing_*.dmg)"
    export TRACEWING_E2E_DMG
    echo "packages: $TRACEWING_E2E_DMG"
fi

export TRACEWING_E2E=1
TRACEWING_E2E_BIN="$(pwd)/$target/release/tracewing"
export TRACEWING_E2E_BIN
if [ -n "$attended" ]; then
    export TRACEWING_E2E_ATTENDED=1
fi

# Each test program, with what it says as it goes, and a time it may take:
# one that hangs is ended, and what it said up to there is in the log,
# rather than the whole job waiting for its own limit and saying nothing.
# macOS has no timeout(1); perl's alarm ends the program it execs.
limit=${E2E_LIMIT:-900}
limited() {
    perl -e 'alarm shift; exec @ARGV' "$limit" "$1" --test-threads=1 --nocapture
}

status=0
for test in $tests; do
    name=$(basename "$test")
    echo "== $name"
    case "$name" in
    # pf takes an administrator: as root when nobody is here to be asked.
    quic_block-*)
        if [ -n "$attended" ]; then
            limited "$test" || status=1
        else
            sudo -E perl -e 'alarm shift; exec @ARGV' "$limit" "$test" --test-threads=1 --nocapture || status=1
        fi
        ;;
    *) limited "$test" || status=1 ;;
    esac
done
exit $status
