#!/bin/sh
# One desktop session, as the tests' user, inside dbus-run-session: what a
# login starts — an unlocked keyring answering on the session bus — then
# every test program in /e2e/tests, one at a time, all of them even when one
# fails.
set -u
printf e2e | gnome-keyring-daemon --unlock --components=secrets >/dev/null || exit 1
status=0
for t in /e2e/tests/*; do
    "$t" --test-threads=1 || status=1
done
exit $status
