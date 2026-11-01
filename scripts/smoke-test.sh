#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
binary=${1:-target/debug/monolito-greeter}
test -x "$binary"
# xvfb-run's wrapper hardcodes /usr/bin/Xvfb on some distributions; use PATH.
display_number=97
while [ -e "/tmp/.X${display_number}-lock" ]; do display_number=$((display_number + 1)); done
Xvfb ":$display_number" -screen 0 1280x800x24 -nolisten tcp > /tmp/monolito-xvfb.log 2>&1 &
xvfb_pid=$!
trap 'kill "$xvfb_pid" 2>/dev/null || true' EXIT HUP INT TERM
export DISPLAY=":$display_number"
export GDK_BACKEND=x11
export GSK_RENDERER=cairo
# No accessibility bus in this isolated display. The desktop runtime keeps it enabled.
export GTK_A11Y=none
attempt=0
until xdpyinfo >/dev/null 2>&1; do
    attempt=$((attempt + 1))
    if [ "$attempt" -gt 50 ]; then cat /tmp/monolito-xvfb.log >&2; exit 1; fi
    sleep 0.1
done
dbus-run-session -- "$binary" --demo --smoke-test
