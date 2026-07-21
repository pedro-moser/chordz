#!/usr/bin/env bash
# Records the web app with its real audio, without taking over the desktop.
#
#   Xvfb          virtual X display; Chromium runs "headed" there, so it keeps a
#                 normal audio pipeline (true headless discards audio).
#   null-sink     a fake sound card; only this browser plays into it, so the
#                 capture never picks up anything else on the machine.
#   ffmpeg        one process, two inputs, one file: no A/V sync to fix later.
#
# Usage: record.sh out.mkv driver.mjs
set -euo pipefail

OUT=${1:?uso: record.sh saida.mkv driver.mjs}
DRIVER=${2:?uso: record.sh saida.mkv driver.mjs}

DISPLAY_NUM=${DISPLAY_NUM:-99}
W=${CAPTURE_W:-1600}
H=${CAPTURE_H:-1000}
FPS=${CAPTURE_FPS:-30}
SINK=chordz_cap

# Resolve OUT/DRIVER to absolute paths so this script works no matter which
# directory it was invoked from.
mkdir -p "$(dirname "$OUT")"
OUT=$(realpath -m "$OUT")
DRIVER=$(realpath "$DRIVER")

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
WEB_DIR=$(realpath "$SCRIPT_DIR/../../web")

# Drivers `import { chromium } from 'playwright'`, and playwright only lives
# in web/node_modules. Node's ESM resolver does NOT honor NODE_PATH (that is
# a CommonJS-only mechanism) and does NOT use the process's cwd either: it
# walks up the *importing file's own* directory tree looking for
# node_modules. tools/capture/ is a sibling of web/, not an ancestor, so
# neither env var nor `cd` fixes it -- only a node_modules link that lives
# alongside the driver does. Create it once, self-healing if missing.
# -sfn, not -s: with a broken link (web/node_modules deleted) `-e` follows it,
# reports "missing", and a bare `ln -s` then fails with "File exists".
ln -sfn "$WEB_DIR/node_modules" "$SCRIPT_DIR/node_modules"

# The trap is installed BEFORE anything is started, and cleanup tolerates every
# variable being unset. Registering it later leaves a window where a failure
# (say pactl refusing the module) kills the script under `set -e` with Xvfb
# already forked: the orphan keeps /tmp/.X99-lock and the NEXT take fails for a
# reason unrelated to its real cause.
XVFB_PID=""; MODULE_ID=""; FFMPEG_PID=""
cleanup() {
  [ -n "$FFMPEG_PID" ] && { kill -INT "$FFMPEG_PID" 2>/dev/null || true; wait "$FFMPEG_PID" 2>/dev/null || true; }
  [ -n "$XVFB_PID" ] && { kill "$XVFB_PID" 2>/dev/null || true; }
  [ -n "$MODULE_ID" ] && { pactl unload-module "$MODULE_ID" 2>/dev/null || true; }
  [ -n "${PROGRESS:-}" ] && rm -f "$PROGRESS"
  return 0
}
trap cleanup EXIT

Xvfb ":$DISPLAY_NUM" -screen 0 "${W}x${H}x24" -nolisten tcp &
XVFB_PID=$!

# node.pause-on-idle=false is load-bearing: without it PipeWire suspends the
# null sink while nothing is playing into it, and ffmpeg, already attached to
# the monitor, keeps reading digital silence after playback starts. The take
# then measures -91 dB with no error anywhere.
MODULE_ID=$(pactl load-module module-null-sink \
  sink_name=$SINK \
  sink_properties="device.description=chordz_capture node.pause-on-idle=false")

# Poll for readiness instead of sleeping a guessed amount. A fixed sleep that
# expires early means ffmpeg starts before the sink exists; one that expires
# late just wastes time. Both are avoidable.
# Xvfb creates its unix socket once the display is actually serving; checking
# for the socket needs no extra package (xdpyinfo/xset are not installed here).
XSOCK="/tmp/.X11-unix/X${DISPLAY_NUM}"
for _ in $(seq 1 50); do
  [ -S "$XSOCK" ] && pactl list short sinks | grep -q "$SINK" && break
  sleep 0.1
done
[ -S "$XSOCK" ] || { echo "Xvfb :$DISPLAY_NUM nao subiu ($XSOCK ausente)" >&2; exit 1; }
pactl list short sinks | grep -q "$SINK" || { echo "sink $SINK nao subiu" >&2; exit 1; }

PROGRESS=$(mktemp)
ffmpeg -hide_banner -loglevel warning -y \
  -progress "$PROGRESS" \
  -f x11grab -framerate "$FPS" -video_size "${W}x${H}" -i ":$DISPLAY_NUM" \
  -f pulse -i "${SINK}.monitor" \
  -c:v libx264 -preset ultrafast -crf 16 -pix_fmt yuv420p \
  -c:a flac \
  "$OUT" &
FFMPEG_PID=$!

# Wait for ffmpeg to report progress: that means it opened both inputs and is
# encoding. Do NOT gate on the output file growing: mkv buffers, and the file
# can stay at zero bytes for over ten seconds while the capture is perfectly
# healthy. Starting the driver too early loses the attack of the first note.
for _ in $(seq 1 150); do
  grep -q "out_time_ms" "$PROGRESS" 2>/dev/null && break
  sleep 0.1
done
grep -q "out_time_ms" "$PROGRESS" 2>/dev/null || { echo "ffmpeg nao reportou progresso" >&2; exit 1; }

DISPLAY=":$DISPLAY_NUM" PULSE_SINK="$SINK" node "$DRIVER"

# Flush the muxer: -INT makes ffmpeg finalize the file instead of truncating it.
kill -INT "$FFMPEG_PID" 2>/dev/null || true
wait "$FFMPEG_PID" 2>/dev/null || true
FFMPEG_PID=""

echo "gravado: $OUT"
