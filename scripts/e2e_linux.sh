#!/bin/sh
# Runs crates/e2e's tests, the ones that change the computer they run on, in a
# container that may be changed (crates/e2e/linux). tracewing and the tests
# are built here and handed to it as they are.
#
#   scripts/e2e_linux.sh            # build, then run every scenario
#   E2E_SKIP_BUILD=1 scripts/e2e_linux.sh   # run what was built last
#
# Needs docker, and python3 to read cargo's list of what it built.
set -eu
cd "$(dirname "$0")/.."
target=${CARGO_TARGET_DIR:-target}

stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
mkdir "$stage/bin" "$stage/tests"

if [ -z "${E2E_SKIP_BUILD:-}" ]; then
    cargo build --release --locked -p tracewing
fi
cargo test --release --locked -p tracewing-e2e --no-run --message-format=json \
    | python3 -I -c '
import json, shutil, sys, os
stage = sys.argv[1]
for line in sys.stdin:
    msg = json.loads(line)
    if msg.get("reason") == "compiler-artifact" and msg.get("executable") \
            and "test" in msg["target"]["kind"]:
        shutil.copy(msg["executable"], os.path.join(stage, msg["target"]["name"]))
' "$stage/tests"
cp "$target/release/tracewing" "$stage/bin/tracewing"
# The tests run as an ordinary user in there, who must read what is here.
chmod -R a+rX "$stage"
echo "tests: $(ls "$stage/tests" | tr '\n' ' ')"

docker build --quiet --tag tracewing-e2e-linux crates/e2e/linux
docker run --rm --cap-add NET_ADMIN --volume "$stage:/e2e:ro" tracewing-e2e-linux
