#!/bin/bash
#
# Packages a release build of tracewing for macOS into dist/: tracewing.app in
# a disk image (docs/releasing.md).
#
#   cargo build --release --locked -p tracewing
#   scripts/package_macos.sh
#
#   dist/tracewing_0.1.0_macos_aarch64.dmg
#
# One file is the program: the window, the proxy, the command line and the MCP
# server are all tracewing, in the bundle's MacOS folder. A terminal can run it
# from there, or from /Applications once the app is moved in.
#
# Signed with a Developer ID and notarized where this machine has one; without
# one, signed ad hoc, and Gatekeeper asks the first time the app opens
# (Control-click > Open).
#
# The licenses of the Rust crates tracewing is built with go in the app's
# Resources, written by cargo-about (about.toml, packaging/licenses.hbs):
#
#   cargo install cargo-about --locked --features cli
#
# Environment: BIN (default target/release/tracewing), ARCH (default the
# machine's: aarch64 or x86_64), DIST (default dist). To sign for people
# (docs/releasing.md):
#
#   MACOS_SIGN_IDENTITY         "Developer ID Application: Name (TEAMID)", in a
#                               keychain this user can use; none: ad hoc
#   APPLE_API_KEY_PATH, APPLE_API_KEY_ID, APPLE_API_ISSUER_ID
#                               an App Store Connect API key, to notarize

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

VERSION="$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' Cargo.toml)"
ARCH="${ARCH:-$(uname -m | sed 's/arm64/aarch64/')}"
BIN="${BIN:-target/release/tracewing}"
DIST="${DIST:-dist}"
WORK="target/package-macos"

case "$ARCH" in
aarch64) TARGET=aarch64-apple-darwin ;;
x86_64) TARGET=x86_64-apple-darwin ;;
*) echo "no macOS package for $ARCH" >&2; exit 1 ;;
esac
[ -x "$BIN" ] || { echo "no build at $BIN: cargo build --release -p tracewing" >&2; exit 1; }
cargo about --version >/dev/null 2>&1 || { echo "no cargo-about: cargo install cargo-about --locked --features cli" >&2; exit 1; }
rm -rf "$WORK"
mkdir -p "$WORK" "$DIST"

IDENTITY="${MACOS_SIGN_IDENTITY:-}"

## The icon, from the PNGs
ICONSET="$WORK/tracewing.iconset"
mkdir -p "$ICONSET"
for size in 16 32 128 256 512; do
	cp "packaging/icons/png/tracewing-$size.png" "$ICONSET/icon_${size}x${size}.png"
	cp "packaging/icons/png/tracewing-$((size * 2)).png" "$ICONSET/icon_${size}x${size}@2x.png"
done
iconutil -c icns -o "$WORK/tracewing.icns" "$ICONSET"

## The licenses of the crates built into the program for this Mac
cargo about generate --locked --fail -c about.toml -m crates/app/Cargo.toml \
	--target "$TARGET" -o "$WORK/THIRD-PARTY-LICENSES.txt" packaging/licenses.hbs

## tracewing.app
APP="$WORK/tracewing.app"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/tracewing"
strip -x "$APP/Contents/MacOS/tracewing"
cp "$WORK/tracewing.icns" "$APP/Contents/Resources/tracewing.icns"
cp LICENSE "$APP/Contents/Resources/LICENSE.txt"
cp "$WORK/THIRD-PARTY-LICENSES.txt" "$APP/Contents/Resources/THIRD-PARTY-LICENSES.txt"
sed "s/@VERSION@/$VERSION/g" packaging/macos/Info.plist >"$APP/Contents/Info.plist"
plutil -lint "$APP/Contents/Info.plist"

## Signed: with the Developer ID where there is one, ad hoc where there is not
if [ -z "$IDENTITY" ]; then
	echo "warning: no MACOS_SIGN_IDENTITY: signed ad hoc, and Gatekeeper will ask the first time it opens" >&2
	codesign --force --sign - --timestamp=none "$APP"
else
	# The hardened runtime and a secure timestamp, as notarizing requires.
	codesign --force --sign "$IDENTITY" --options runtime --timestamp "$APP"
fi
codesign --verify --deep --strict "$APP"

# Apple checks what it is given and keeps a ticket for it, stapled here so
# Gatekeeper lets it open offline too.
notarize() {
	local file="$1" result status id
	result="$(xcrun notarytool submit "$file" --wait --output-format json \
		--key "$APPLE_API_KEY_PATH" --key-id "$APPLE_API_KEY_ID" --issuer "$APPLE_API_ISSUER_ID")"
	status="$(plutil -extract status raw - <<<"$result" 2>/dev/null || true)"
	if [ "$status" != "Accepted" ]; then
		echo "notarizing $file: ${status:-no answer}" >&2
		id="$(plutil -extract id raw - <<<"$result" 2>/dev/null || true)"
		[ -z "$id" ] || xcrun notarytool log "$id" \
			--key "$APPLE_API_KEY_PATH" --key-id "$APPLE_API_KEY_ID" --issuer "$APPLE_API_ISSUER_ID" >&2
		exit 1
	fi
}
# Stapling fetches the ticket from Apple's CloudKit, and a dropped connection
# there is no reason to lose a build Apple has already accepted.
staple() {
	local try
	for try in 1 2 3 4 5; do
		if xcrun stapler staple "$1"; then
			return 0
		fi
		echo "stapling $1 failed (try $try of 5)" >&2
		[ "$try" -lt 5 ] && sleep 30
	done
	return 1
}
NOTARIZE=""
if [ -n "$IDENTITY" ] && [ -n "${APPLE_API_KEY_PATH:-}" ]; then
	NOTARIZE=1
elif [ -n "$IDENTITY" ]; then
	echo "warning: not notarized (no APPLE_API_KEY_PATH): Gatekeeper refuses it downloaded" >&2
fi

## Notarized, the app first: the disk image carries it with its ticket.
if [ -n "$NOTARIZE" ]; then
	ditto -c -k --keepParent "$APP" "$WORK/tracewing.zip"
	notarize "$WORK/tracewing.zip"
	staple "$APP"
	spctl --assess --type execute --verbose=2 "$APP"
fi

## A disk image with the app and a link to /Applications
STAGE="$WORK/dmg"
mkdir -p "$STAGE"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
DMG="$DIST/tracewing_${VERSION}_macos_${ARCH}.dmg"
hdiutil create -volname "tracewing $VERSION" -srcfolder "$STAGE" -fs HFS+ -format UDZO -ov "$DMG"

## The disk image signed, and notarized too.
if [ -n "$IDENTITY" ]; then
	codesign --force --sign "$IDENTITY" --timestamp "$DMG"
fi
if [ -n "$NOTARIZE" ]; then
	notarize "$DMG"
	staple "$DMG"
fi

ls -l "$DIST"
