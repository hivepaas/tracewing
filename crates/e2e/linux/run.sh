#!/bin/sh
# Inside the container, as root: start what a desktop session has running,
# then run each test program given in /e2e/tests as the tests' user, once as
# GNOME and once as KDE. tracewing itself is /e2e/bin/tracewing, and its
# package for this system is in /e2e/dist.
set -eu

# The package this system installs: the .deb where there is dpkg, the .rpm
# elsewhere. scripts/e2e_linux.sh always hands one in; a stage made by hand
# for a few tests may have none, and then the package's test says so.
if command -v dpkg >/dev/null; then kind=deb; else kind=rpm; fi
package=$(ls /e2e/dist/*.$kind 2>/dev/null | head -n 1 || true)

mkdir -p /run/dbus
dbus-daemon --system --fork
/usr/lib/polkit-1/polkitd --no-debug >/tmp/polkitd.log 2>&1 &
# polkitd answers on the system bus once it has read its rules.
for _ in $(seq 50); do
    if gdbus introspect --system --dest org.freedesktop.PolicyKit1 \
        --object-path /org/freedesktop/PolicyKit1/Authority >/dev/null 2>&1; then
        break
    fi
    sleep 0.1
done

status=0
for desktop in GNOME KDE; do
    echo "== as $desktop"
    # What a Plasma 6 session sets beside XDG_CURRENT_DESKTOP, and what
    # Chrome reads to know which KDE's settings to read: without it, it
    # looks for KDE 4's, in a folder Plasma no longer keeps.
    session_version=
    [ "$desktop" = KDE ] && session_version=6
    # A session bus of its own for each, as a login has; dconf keeps GNOME's
    # settings in the user's home, and kioslaverc is there too.
    if ! su tester -c "env TRACEWING_E2E=1 TRACEWING_E2E_BIN=/e2e/bin/tracewing \
        TRACEWING_E2E_PACKAGE=$package \
        TRACEWING_E2E_BROWSERS='${TRACEWING_E2E_BROWSERS:-}' \
        XDG_CURRENT_DESKTOP=$desktop KDE_SESSION_VERSION=$session_version \
        GSETTINGS_BACKEND=dconf \
        dbus-run-session -- /usr/local/bin/tracewing-e2e-session"; then
        status=1
    fi
done
exit $status
