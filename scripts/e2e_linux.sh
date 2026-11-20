#!/bin/sh
# Runs crates/e2e's tests, the ones that change the computer they run on, in
# containers that may be changed (crates/e2e/linux): Ubuntu, which installs
# the .deb, and Fedora, which installs the .rpm. tracewing, its packages and
# the tests are built here and handed to them as they are.
#
#   scripts/e2e_linux.sh            # build, then run every scenario on both
#   E2E_SKIP_BUILD=1 scripts/e2e_linux.sh   # run what was built last
#   E2E_SYSTEMS=ubuntu scripts/e2e_linux.sh # on one of them
#
# Needs docker, python3 to read cargo's list of what it built, and what
# scripts/package_linux.sh needs: cargo-about, dpkg-dev and rpm.
set -eu
cd "$(dirname "$0")/.."
target=${CARGO_TARGET_DIR:-target}
dist="$target/e2e-dist"
systems=${E2E_SYSTEMS:-ubuntu fedora}

stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
mkdir "$stage/bin" "$stage/tests" "$stage/dist"

if [ -z "${E2E_SKIP_BUILD:-}" ]; then
    cargo build --release --locked -p tracewing
    rm -rf "$dist"
    BIN="$target/release/tracewing" DIST="$dist" scripts/package_linux.sh
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
cp "$dist"/*.deb "$dist"/*.rpm "$stage/dist/"
# The tests run as an ordinary user in there, who must read what is here.
chmod -R a+rX "$stage"
echo "tests: $(ls "$stage/tests" | tr '\n' ' ')"
echo "packages: $(ls "$stage/dist" | tr '\n' ' ')"

# A tag of this checkout's own: another checkout of the repository on this
# computer builds its image from its own files, and two runs at once would
# otherwise each start whichever image was built last.
checkout=$(pwd -P | cksum | cut -d' ' -f1)

status=0
for system in $systems; do
    case "$system" in
    ubuntu) dockerfile=Dockerfile ;;
    fedora) dockerfile=Dockerfile.fedora ;;
    *) echo "no container for $system" >&2; exit 1 ;;
    esac
    echo "=== $system"
    image="tracewing-e2e-$system-$checkout"
    docker build --quiet --tag "$image" \
        --file "crates/e2e/linux/$dockerfile" crates/e2e/linux
    docker run --rm --cap-add NET_ADMIN --volume "$stage:/e2e:ro" "$image" ||
        status=1
done
exit $status
