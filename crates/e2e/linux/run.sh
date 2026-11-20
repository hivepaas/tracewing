#!/bin/sh
# Inside the container, as root: start what a desktop session has running,
# then run each test program given in /e2e/tests as the tests' user, once as
# GNOME and once as KDE. tracewing itself is /e2e/bin/tracewing.
set -eu

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
    # A session bus of its own for each, as a login has; dconf keeps GNOME's
    # settings in the user's home, and kioslaverc is there too.
    if ! su tester -c "env TRACEWING_E2E=1 TRACEWING_E2E_BIN=/e2e/bin/tracewing \
        XDG_CURRENT_DESKTOP=$desktop GSETTINGS_BACKEND=dconf \
        dbus-run-session -- sh -c 'for t in /e2e/tests/*; do \"\$t\" --test-threads=1 || exit 1; done'"; then
        status=1
    fi
done
exit $status
