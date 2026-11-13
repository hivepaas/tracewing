#!/bin/bash
#
# Checks what scripts/package_macos.sh made: the disk image opened as a person
# opens it, the app inside it read, and the program run from it with no window.
# CI runs it on each Mac it builds for; it runs on a Mac here too.
#
#   scripts/check_macos_app.sh dist/tracewing_0.1.0_macos_aarch64.dmg
#
# The program is run from inside the image, which is read-only and signed as
# the app in it: that it runs at all from there is what says the bundle is
# whole and the signature is one macOS accepts.

set -euo pipefail

DMG="${1:?the .dmg to check}"
[ -f "$DMG" ] || { echo "no file at $DMG" >&2; exit 1; }
DMG="$(cd "$(dirname "$DMG")" && pwd)/$(basename "$DMG")"

FAILED=0
wrong() { echo "  $1" >&2; FAILED=1; }

MOUNT="$(mktemp -d)"
cleanup() {
	hdiutil detach "$MOUNT" -quiet -force 2>/dev/null || true
	rm -rf "$MOUNT"
}
trap cleanup EXIT

echo "$DMG"
hdiutil attach "$DMG" -mountpoint "$MOUNT" -nobrowse -quiet
APP="$MOUNT/tracewing.app"
[ -d "$APP" ] || { echo "  no tracewing.app in the image" >&2; exit 1; }
# The link people drag it onto.
[ -L "$MOUNT/Applications" ] || wrong "no link to /Applications beside it"

for file in \
	"Contents/MacOS/tracewing" \
	"Contents/Resources/tracewing.icns" \
	"Contents/Resources/LICENSE.txt" \
	"Contents/Resources/THIRD-PARTY-LICENSES.txt" \
	"Contents/Info.plist"; do
	[ -e "$APP/$file" ] || wrong "missing: $file"
done
[ -x "$APP/Contents/MacOS/tracewing" ] || wrong "not executable: Contents/MacOS/tracewing"

# The program is built with these, so their licenses are in the file.
for crate in hudsucker rusqlite rustls tokio gpui-pre zstd; do
	grep -q "^  $crate " "$APP/Contents/Resources/THIRD-PARTY-LICENSES.txt" ||
		wrong "no license for $crate in THIRD-PARTY-LICENSES.txt"
done

# What the Finder and the Dock read: the version it says it is, and the
# oldest macOS it says it runs on.
VERSION="$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' \
	"$(git rev-parse --show-toplevel)/Cargo.toml")"
said="$(plutil -extract CFBundleShortVersionString raw -o - "$APP/Contents/Info.plist")"
[ "$said" = "$VERSION" ] || wrong "the bundle says version $said, not $VERSION"
said="$(plutil -extract CFBundleIdentifier raw -o - "$APP/Contents/Info.plist")"
[ "$said" = "com.hivepaas.tracewing" ] || wrong "the bundle's id is $said"

# Signed, whole, and by whom: ad hoc where this machine has no Developer ID.
codesign --verify --deep --strict "$APP" || wrong "the signature does not verify"
codesign --display --verbose=2 "$APP" 2>&1 | sed -n 's/^Authority=/  signed by /p' |
	head -n 1 || true

[ "$FAILED" = 0 ] || { echo "$DMG is not as it should be" >&2; exit 1; }
echo "  holds what it should"

# Run from the image, with no window: a recording of its own, a HAR out of it,
# and the session listed. Nothing of this Mac's is touched — the CA's key goes
# in the folder, not the keychain, and the system proxy is left alone.
dir="$(mktemp -d)"
"$APP/Contents/MacOS/tracewing" --version
"$APP/Contents/MacOS/tracewing" capture --ca-key-in-file --data-dir "$dir" --seconds 1 --har "$dir/out.har" >/dev/null
grep -q '"creator":{"name":"tracewing"' "$dir/out.har" ||
	{ echo "  the HAR it wrote is not one of its own" >&2; exit 1; }
"$APP/Contents/MacOS/tracewing" sessions --data-dir "$dir" | grep -qE '^[0-9]+ ' ||
	{ echo "  the session it recorded is not listed" >&2; exit 1; }
rm -rf "$dir"
echo "  it runs from the image"
