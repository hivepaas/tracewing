#!/bin/bash
#
# Checks what scripts/package_linux.sh made holds what it is to hold. CI runs
# it on each file it makes; it runs here too, on Linux.
#
#   scripts/check_linux_package.sh dist/tracewing_0.1.0_amd64.deb
#   scripts/check_linux_package.sh dist/tracewing_0.1.0_x86_64.rpm
#
# A package: the program, the launcher and icon, the licenses, and the
# libraries it depends on by their packages' names.
#
# With --install, the package is installed as a user does and then the program
# is run from it, with no window: a recording of its own, a HAR written, and
# the session listed. The .deb on this system, with sudo; the .rpm in a Fedora
# container, with Docker.

set -euo pipefail

INSTALL=false
ROUND_TRIP=false
case "${1:-}" in
--install) INSTALL=true; shift ;;
# What the Fedora container runs, with the package installed.
--round-trip) ROUND_TRIP=true; shift ;;
esac
FILE="${1:?the .deb or .rpm to check}"
$ROUND_TRIP || [ -f "$FILE" ] || { echo "no file at $FILE" >&2; exit 1; }

FAILED=0
fail() { echo "  missing: $1" >&2; FAILED=1; }
wrong() { echo "  $1" >&2; FAILED=1; }
FILES=""
has() { grep -qx "$1" <<<"$FILES" || fail "$1"; }
done_checking() {
	[ "$FAILED" = 0 ] || { echo "$FILE is not as it should be" >&2; exit 1; }
	echo "  holds what it should"
}

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# The program is built with these, so their licenses are in the file: a
# template that stopped writing crates would otherwise pass unnoticed.
licenses_name_the_crates() {
	local crate
	for crate in hudsucker rusqlite rustls tokio gpui-pre zstd; do
		grep -q "^  $crate " "$1" || wrong "no license for $crate in $(basename "$1")"
	done
}

# The program at $1 records with no window: a session of its own, a HAR out of
# it, and the session listed. Nothing of the machine's is touched — the CA's
# key goes in the folder, not the keychain, and the system proxy is left alone.
run_from_the_package() {
	local tracewing="$1" dir
	dir="$(mktemp -d "$WORK/run.XXXX")"
	"$tracewing" --version
	"$tracewing" capture --ca-key-in-file --data-dir "$dir" --seconds 1 --har "$dir/out.har" >/dev/null
	grep -q '"creator":{"name":"tracewing"' "$dir/out.har" ||
		{ echo "  the HAR it wrote is not one of its own" >&2; exit 1; }
	[ -f "$dir/ca.pem" ] || { echo "  it made no certificate authority" >&2; exit 1; }
	"$tracewing" sessions --data-dir "$dir" | grep -qE '^[0-9]+ ' ||
		{ echo "  the session it recorded is not listed" >&2; exit 1; }
	# The one command somebody runs when the network has gone, on a machine
	# where nothing of tracewing's was left behind.
	"$tracewing" recover --data-dir "$dir" | grep -q 'nothing to put back' ||
		{ echo "  recover does not say there is nothing to put back" >&2; exit 1; }
}

check_deb() {
	local listing depends
	listing="$(dpkg-deb -c "$FILE")"
	FILES="$(awk '{print $6}' <<<"$listing" | sed 's|^\./|/|')"
	for file in \
		/usr/bin/tracewing \
		/usr/share/applications/tracewing.desktop \
		/usr/share/icons/hicolor/scalable/apps/tracewing.svg \
		/usr/share/doc/tracewing/copyright \
		/usr/share/doc/tracewing/THIRD-PARTY-LICENSES; do
		has "$file"
	done
	grep -q '^-rwxr-xr-x .* \./usr/bin/tracewing$' <<<"$listing" || wrong "not executable: /usr/bin/tracewing"
	depends="$(dpkg-deb -f "$FILE" Depends)"
	grep -q 'libc6' <<<"$depends" || wrong "Depends has no libc6: $depends"
	dpkg-deb --fsys-tarfile "$FILE" | tar -xf - -C "$WORK" \
		./usr/share/doc/tracewing/THIRD-PARTY-LICENSES ./usr/share/applications/tracewing.desktop
	licenses_name_the_crates "$WORK/usr/share/doc/tracewing/THIRD-PARTY-LICENSES"
	# A launcher the desktop will not read is a program with no icon and no
	# name in the menu.
	if command -v desktop-file-validate >/dev/null; then
		desktop-file-validate "$WORK/usr/share/applications/tracewing.desktop" || wrong "the launcher is not valid"
	fi
	done_checking

	$INSTALL || return 0
	sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends "$(realpath "$FILE")" >/dev/null
	echo "  installed"
	run_from_the_package tracewing
	echo "  it runs from the package"
}

check_rpm() {
	local requires modes rpm
	FILES="$(rpm -qlp "$FILE" 2>/dev/null)"
	for file in \
		/usr/bin/tracewing \
		/usr/share/applications/tracewing.desktop \
		/usr/share/icons/hicolor/scalable/apps/tracewing.svg \
		/usr/share/licenses/tracewing/LICENSE \
		/usr/share/licenses/tracewing/THIRD-PARTY-LICENSES; do
		has "$file"
	done
	# The glibc the program was linked with: a system too old for it refuses
	# the package rather than the program failing when it starts.
	requires="$(rpm -qp --requires "$FILE" 2>/dev/null)"
	grep -q '^libc\.so\.6(GLIBC_' <<<"$requires" || wrong "Requires names no glibc: $requires"
	modes="$(rpm -qp --queryformat '[%{FILEMODES:perms} %{FILENAMES}\n]' "$FILE" 2>/dev/null)"
	grep -q '^-rwxr-xr-x /usr/bin/tracewing$' <<<"$modes" || wrong "not executable: /usr/bin/tracewing"
	rpm="$(realpath "$FILE")"
	(cd "$WORK" && rpm2cpio "$rpm" | cpio -idm --quiet ./usr/share/licenses/tracewing/THIRD-PARTY-LICENSES)
	licenses_name_the_crates "$WORK/usr/share/licenses/tracewing/THIRD-PARTY-LICENSES"
	done_checking

	$INSTALL || return 0
	# Installed as on Fedora, with the dependencies it names; then run.
	local dir name
	dir="$(cd "$(dirname "$FILE")" && pwd)"
	name="$(basename "$FILE")"
	docker run --rm -v "$dir:/dist:ro" -v "$(realpath "$0"):/check.sh:ro" fedora:latest bash -c "
		set -e
		dnf install -y -q /dist/$name grep >/dev/null
		echo '  installed in Fedora'
		/check.sh --round-trip tracewing
		rpm -V tracewing
	"
	echo "  it runs from the package"
}

if $ROUND_TRIP; then
	run_from_the_package "$FILE"
	exit 0
fi
case "$FILE" in
*.deb) echo "$FILE"; check_deb ;;
*.rpm) echo "$FILE"; check_rpm ;;
*) echo "not a .deb or .rpm: $FILE" >&2; exit 1 ;;
esac
