#!/bin/bash
#
# Packages a release build of tracewing for Windows into dist/: the installer,
# and a zip of the program to run from a folder. Run from Git Bash or MSYS2,
# with NSIS 3 installed (or makensis on PATH; on Linux the nsis package's
# works too).
#
#   cargo build --release --locked -p tracewing -p tracewing-console
#   scripts/package_windows.sh
#
#   dist/tracewing_0.1.0_windows_x86_64-setup.exe
#   dist/tracewing_0.1.0_windows_x86_64.zip
#
# One file is the program: the window, the proxy, the command line and the MCP
# server are all tracewing.exe, built with a window and no console. Beside it
# goes tracewing.com (crates/console), built for the console, which a terminal
# finds first for `tracewing`, and which runs tracewing.exe in it and waits.
# The installer puts their folder in the PATH, so a terminal opened after runs
# `tracewing capture` (installer.nsi).
#
# Beside it goes Wintun's own wintun.dll, which the local way in (capture
# with no proxy settings) loads from the program's folder: WireGuard LLC's
# signed build, unchanged, from wintun.net, checked against the SHA-256 it
# publishes, with its licence as WINTUN-LICENSE.txt. Its licence lets it go
# alongside a program that uses it through wintun.h alone, which tracewing
# does (docs/design/capture-modes.md).
#
# Neither is signed (no Authenticode certificate yet): SmartScreen asks the
# first time the installer runs (More info > Run anyway).
#
# Environment: EXE (default target/release/tracewing.exe), COM (default
# tracewing-console.exe beside EXE), ARCH (default
# x86_64), DIST (default dist), MAKENSIS (default: on PATH, else where NSIS
# installs itself), LICENSES (the third-party licenses; default: made by
# cargo-about), WINTUN_ZIP (wintun-0.14.1.zip; default: downloaded once into
# target/).

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

VERSION="$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' Cargo.toml)"
ARCH="${ARCH:-x86_64}"
EXE="$(realpath "${EXE:-target/release/tracewing.exe}")"
COM="$(realpath "${COM:-$(dirname "$EXE")/tracewing-console.exe}")"
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
[ -f "$COM" ] || { echo "no build at $COM: cargo build --release -p tracewing-console" >&2; exit 1; }
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

## Wintun: the release's zip, checked, and from it the DLL for this ARCH and
## the licence that goes with it.
WINTUN_VERSION=0.14.1
WINTUN_SHA256=07c256185d6ee3652e09fa55c0b673e2624b565e02c4b9091c79ca7d2f24ef51
if [ -z "${WINTUN_ZIP:-}" ]; then
	WINTUN_ZIP="target/wintun-$WINTUN_VERSION.zip"
	[ -f "$WINTUN_ZIP" ] || curl -sSfL -o "$WINTUN_ZIP" "https://www.wintun.net/builds/wintun-$WINTUN_VERSION.zip"
fi
echo "$WINTUN_SHA256  $WINTUN_ZIP" | sha256sum -c --quiet - ||
	{ echo "$WINTUN_ZIP is not Wintun $WINTUN_VERSION as wintun.net publishes it" >&2; exit 1; }
case "$ARCH" in
x86_64) WINTUN_ARCH=amd64 ;;
aarch64) WINTUN_ARCH=arm64 ;;
*) echo "no Wintun for $ARCH" >&2; exit 1 ;;
esac
mkdir -p "$WORK/wintun"
if command -v unzip >/dev/null; then
	unzip -q -o "$WINTUN_ZIP" -d "$WORK/wintun"
elif command -v 7z >/dev/null; then
	7z x -y -bso0 "-o$(native "$WORK/wintun")" "$(native "$WINTUN_ZIP")"
else
	powershell.exe -NoProfile -Command "Expand-Archive -Force '$(native "$WINTUN_ZIP")' '$(native "$WORK/wintun")'"
fi
WINTUN_DLL="$(realpath "$WORK/wintun/wintun/bin/$WINTUN_ARCH/wintun.dll")"
WINTUN_LICENSE="$(realpath "$WORK/wintun/wintun/LICENSE.txt")"

## The installer
# The version as Windows writes a file's: four numbers, no beta.
FILEVERSION="${VERSION%%-*}.0"
SETUP="$DIST/tracewing_${VERSION}_windows_${ARCH}-setup.exe"
# The paths are Windows' already: MSYS is not to rewrite them.
MSYS2_ARG_CONV_EXCL='*' "$MAKENSIS" -V3 \
	"-DVERSION=$VERSION" "-DFILEVERSION=$FILEVERSION" \
	"-DEXE=$(native "$EXE")" "-DCOM=$(native "$COM")" "-DLICENSES=$(native "$LICENSES")" \
	"-DWINTUN=$(native "$WINTUN_DLL")" "-DWINTUN_LICENSE=$(native "$WINTUN_LICENSE")" \
	"-DOUT=$(native "$SETUP")" \
	"$(native "$ROOT/packaging/windows/installer.nsi")"

## The zip: the program, Wintun's DLL and the licenses, in a folder of their own. Run from
## there, tracewing keeps everything it records under %LOCALAPPDATA%, as the
## installed one does.
NAME="tracewing-${VERSION}-windows-${ARCH}"
mkdir -p "$WORK/$NAME"
cp "$EXE" "$WORK/$NAME/tracewing.exe"
cp "$COM" "$WORK/$NAME/tracewing.com"
cp LICENSE "$WORK/$NAME/LICENSE.txt"
cp "$LICENSES" "$WORK/$NAME/THIRD-PARTY-LICENSES.txt"
cp "$WINTUN_DLL" "$WORK/$NAME/wintun.dll"
cp "$WINTUN_LICENSE" "$WORK/$NAME/WINTUN-LICENSE.txt"
ZIP="$DIST/tracewing_${VERSION}_windows_${ARCH}.zip"
rm -f "$ZIP"
if command -v 7z >/dev/null; then
	(cd "$WORK" && 7z a -tzip -bso0 "$(native "$ZIP")" "$NAME")
else
	(cd "$WORK" && zip -qr "$ZIP" "$NAME")
fi

ls -l "$DIST"
