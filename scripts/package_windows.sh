#!/bin/bash
#
# Packages a release build of tracewing for Windows into dist/: the installer,
# and a zip of the program to run from a folder. Run from Git Bash or MSYS2,
# with NSIS 3 installed (or makensis on PATH; on Linux the nsis package's
# works too).
#
#   cargo build --release --locked -p tracewing
#   scripts/package_windows.sh
#
#   dist/tracewing_0.1.0_windows_x86_64-setup.exe
#   dist/tracewing_0.1.0_windows_x86_64.zip
#
# One file is the program: the window, the proxy, the command line and the MCP
# server are all tracewing.exe. The installer puts its folder in the PATH, so a
# terminal opened after runs `tracewing capture` (installer.nsi).
#
# Neither is signed (no Authenticode certificate yet): SmartScreen asks the
# first time the installer runs (More info > Run anyway).
#
# Environment: EXE (default target/release/tracewing.exe), ARCH (default
# x86_64), DIST (default dist), MAKENSIS (default: on PATH, else where NSIS
# installs itself), LICENSES (the third-party licenses; default: made by
# cargo-about).

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

VERSION="$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' Cargo.toml)"
ARCH="${ARCH:-x86_64}"
EXE="$(realpath "${EXE:-target/release/tracewing.exe}")"
DIST="${DIST:-dist}"
WORK="target/package-windows"

if [ -z "${MAKENSIS:-}" ]; then
	MAKENSIS="$(command -v makensis || true)"
	for dir in "/c/Program Files (x86)/NSIS" "/c/Program Files/NSIS"; do
		[ -n "$MAKENSIS" ] || { [ -x "$dir/makensis.exe" ] && MAKENSIS="$dir/makensis.exe"; } || true
	done
fi
[ -n "$MAKENSIS" ] || { echo "no makensis: install NSIS 3 (choco install nsis)" >&2; exit 1; }
[ -f "$EXE" ] || { echo "no build at $EXE: cargo build --release -p tracewing" >&2; exit 1; }
rm -rf "$WORK"
mkdir -p "$WORK" "$DIST"
DIST="$(realpath "$DIST")"

# A path as the tool takes it: Windows' own under MSYS, as is elsewhere.
native() { if command -v cygpath >/dev/null; then cygpath -w "$1"; else echo "$1"; fi; }

## The licenses of the crates built into the program for Windows
if [ -z "${LICENSES:-}" ]; then
	cargo about --version >/dev/null 2>&1 || { echo "no cargo-about: cargo install cargo-about --locked --features cli" >&2; exit 1; }
	LICENSES="$WORK/THIRD-PARTY-LICENSES.txt"
	cargo about generate --locked --fail -c about.toml -m crates/app/Cargo.toml \
		--target "$ARCH-pc-windows-msvc" -o "$LICENSES" packaging/licenses.hbs
fi
LICENSES="$(realpath "$LICENSES")"

## The installer
# The version as Windows writes a file's: four numbers, no beta.
FILEVERSION="${VERSION%%-*}.0"
SETUP="$DIST/tracewing_${VERSION}_windows_${ARCH}-setup.exe"
# The paths are Windows' already: MSYS is not to rewrite them.
MSYS2_ARG_CONV_EXCL='*' "$MAKENSIS" -V3 \
	"-DVERSION=$VERSION" "-DFILEVERSION=$FILEVERSION" \
	"-DEXE=$(native "$EXE")" "-DLICENSES=$(native "$LICENSES")" \
	"-DOUT=$(native "$SETUP")" \
	"$(native "$ROOT/packaging/windows/installer.nsi")"

## The zip: the program and its licenses, in a folder of their own. Run from
## there, tracewing keeps everything it records under %LOCALAPPDATA%, as the
## installed one does.
NAME="tracewing-${VERSION}-windows-${ARCH}"
mkdir -p "$WORK/$NAME"
cp "$EXE" "$WORK/$NAME/tracewing.exe"
cp LICENSE "$WORK/$NAME/LICENSE.txt"
cp "$LICENSES" "$WORK/$NAME/THIRD-PARTY-LICENSES.txt"
ZIP="$DIST/tracewing_${VERSION}_windows_${ARCH}.zip"
rm -f "$ZIP"
if command -v 7z >/dev/null; then
	(cd "$WORK" && 7z a -tzip -bso0 "$(native "$ZIP")" "$NAME")
else
	(cd "$WORK" && zip -qr "$ZIP" "$NAME")
fi

ls -l "$DIST"
