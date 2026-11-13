#!/bin/bash
#
# Packages a release build of tracewing for Linux into dist/, as a .deb and an
# .rpm: the program, its launcher and icon, and the licenses of what it is
# built with.
#
#   cargo build --release --locked -p tracewing
#   scripts/package_linux.sh
#
#   dist/tracewing_0.1.0_amd64.deb
#   dist/tracewing_0.1.0_x86_64.rpm
#
# One binary holds everything: the window, the proxy, the command line and the
# MCP server. There is nothing else to pack.
#
# The licenses of the Rust crates the program is built with go in its doc
# folder, written by cargo-about (about.toml, packaging/licenses.hbs):
#
#   cargo install cargo-about --locked --features cli
#
# Environment: BIN (default target/release/tracewing), ARCH (default the
# machine's: x86_64 or aarch64), DIST (default dist).

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

VERSION="$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' Cargo.toml)"
ARCH="${ARCH:-$(uname -m)}"
BIN="${BIN:-target/release/tracewing}"
DIST="${DIST:-dist}"
WORK="target/package-linux"

case "$ARCH" in
x86_64) DEB_ARCH=amd64 ;;
aarch64) DEB_ARCH=arm64 ;;
*) echo "no Linux package for $ARCH" >&2; exit 1 ;;
esac

[ -x "$BIN" ] || { echo "no build at $BIN: cargo build --release -p tracewing" >&2; exit 1; }
cargo about --version >/dev/null 2>&1 || { echo "no cargo-about: cargo install cargo-about --locked --features cli" >&2; exit 1; }
rm -rf "$WORK"
mkdir -p "$WORK" "$DIST"

DEB="$WORK/deb"
install -Dm755 "$BIN" "$DEB/usr/bin/tracewing"
strip "$DEB/usr/bin/tracewing"
install -Dm644 packaging/linux/tracewing.desktop "$DEB/usr/share/applications/tracewing.desktop"
install -Dm644 packaging/icons/tracewing.svg "$DEB/usr/share/icons/hicolor/scalable/apps/tracewing.svg"
install -Dm644 packaging/linux/copyright "$DEB/usr/share/doc/tracewing/copyright"
# Those of the crates built into the program for this machine; one under a
# license about.toml does not accept stops here.
cargo about generate --locked --fail -c about.toml -m crates/app/Cargo.toml \
	--target "$ARCH-unknown-linux-gnu" -o "$WORK/THIRD-PARTY-LICENSES" packaging/licenses.hbs
install -Dm644 "$WORK/THIRD-PARTY-LICENSES" "$DEB/usr/share/doc/tracewing/THIRD-PARTY-LICENSES"

# The libraries the program links, as the system names their packages.
mkdir -p "$WORK/shlibs/debian"
printf 'Source: tracewing\n\nPackage: tracewing\nArchitecture: any\n' >"$WORK/shlibs/debian/control"
DEPENDS="$(cd "$WORK/shlibs" && dpkg-shlibdeps -O "$ROOT/$DEB/usr/bin/tracewing" 2>/dev/null | sed -n 's/^shlibs:Depends=//p')"
mkdir -p "$DEB/DEBIAN"
cat >"$DEB/DEBIAN/control" <<EOF
Package: tracewing
Version: $VERSION
Architecture: $DEB_ARCH
Maintainer: HivePaaS <support@hivepaas.com>
Installed-Size: $(du -sk --exclude=DEBIAN "$DEB" | cut -f1)
Depends: $DEPENDS
Recommends: libwayland-client0, libxcb1, libx11-xcb1, libvulkan1, mesa-vulkan-drivers
Section: devel
Priority: optional
Homepage: https://tracewing.app
Description: HTTP(S) debugging proxy that keeps up with big sessions
 Shows what a program asks of the network and what it is answered: mobile
 apps, desktop apps, servers and AI agents, where a browser's own tools
 cannot look. Requests can be held, edited and sent on, or answered from a
 rule without reaching anybody. One window, no browser extension, and a
 session of a million flows stays as quick as an empty one.
EOF
chmod -R go-w "$DEB"
dpkg-deb --root-owner-group --build "$DEB" "$DIST/tracewing_${VERSION}_${DEB_ARCH}.deb"

## The .rpm, for Fedora and openSUSE: the .deb's files where those systems
## keep them. rpmbuild works out what the program needs from them, the glibc
## version included, so a system too old for it refuses the package at install
## rather than tracewing failing when it starts.
RPM="$WORK/rpm"
RPMROOT="$ROOT/$WORK/rpm-root"
install -Dm755 "$DEB/usr/bin/tracewing" "$RPMROOT/usr/bin/tracewing"
install -Dm644 packaging/linux/tracewing.desktop "$RPMROOT/usr/share/applications/tracewing.desktop"
install -Dm644 packaging/icons/tracewing.svg "$RPMROOT/usr/share/icons/hicolor/scalable/apps/tracewing.svg"
install -Dm644 LICENSE "$RPMROOT/usr/share/licenses/tracewing/LICENSE"
install -Dm644 "$WORK/THIRD-PARTY-LICENSES" "$RPMROOT/usr/share/licenses/tracewing/THIRD-PARTY-LICENSES"
mkdir -p "$RPM"
cat >"$RPM/tracewing.spec" <<SPEC
%global debug_package %{nil}
%global _build_id_links none
%global __brp_strip %{nil}
Name: tracewing
# An RPM version has no hyphen; ~ sorts a beta before its release.
Version: ${VERSION/-/\~}
Release: 1
Summary: HTTP(S) debugging proxy that keeps up with big sessions
License: GPL-3.0-or-later
URL: https://tracewing.app
Recommends: libwayland-client, libxcb, libX11-xcb, vulkan-loader, mesa-vulkan-drivers

%description
Shows what a program asks of the network and what it is answered: mobile
apps, desktop apps, servers and AI agents, where a browser's own tools
cannot look. Requests can be held, edited and sent on, or answered from a
rule without reaching anybody. One window, no browser extension, and a
session of a million flows stays as quick as an empty one.

%install
cp -a "$RPMROOT/." "%{buildroot}/"

%files
/usr/bin/tracewing
/usr/share/applications/tracewing.desktop
/usr/share/icons/hicolor/scalable/apps/tracewing.svg
%license /usr/share/licenses/tracewing/LICENSE
%license /usr/share/licenses/tracewing/THIRD-PARTY-LICENSES
SPEC
rpmbuild -bb --quiet --target "$ARCH" --define "_topdir $ROOT/$RPM" "$RPM/tracewing.spec"
cp "$RPM/RPMS/$ARCH/"*.rpm "$DIST/tracewing_${VERSION}_${ARCH}.rpm"

ls -l "$DIST"
