#!/bin/sh
# Requires Xvfb, xdotool and ImageMagick. Captures only the demo window.
set -eu
cd "$(dirname "$0")/.."
mkdir -p artifacts
display_number=98
while [ -e "/tmp/.X${display_number}-lock" ]; do display_number=$((display_number + 1)); done
Xvfb ":$display_number" -screen 0 1440x1000x24 -nolisten tcp > /tmp/monolito-capture-xvfb.log 2>&1 &
xvfb_pid=$!
app_pid=
trap 'test -z "$app_pid" || kill "$app_pid" 2>/dev/null || true; kill "$xvfb_pid" 2>/dev/null || true' EXIT HUP INT TERM
export DISPLAY=":$display_number" GDK_BACKEND=x11 GSK_RENDERER=cairo GTK_A11Y=none
sleep 0.3
target/debug/monolito-greeter --demo > artifacts/demo.log 2>&1 &
app_pid=$!
attempt=0
window=
while [ -z "$window" ]; do
    window=$(xdotool search --onlyvisible --name 'Monolito · Entrar' 2>/dev/null | head -1 || true)
    attempt=$((attempt + 1))
    if [ "$attempt" -gt 50 ]; then cat artifacts/demo.log >&2; exit 1; fi
    sleep 0.1
done
sleep 0.5
import -window "$window" artifacts/01-descanso.png
xdotool windowfocus --sync "$window" key space
sleep 0.5
import -window "$window" artifacts/02-login.png
xdotool windowsize "$window" 1024 768
sleep 0.5
import -window "$window" artifacts/03-login-1024.png
echo 'Capturas salvas em artifacts/'
