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
if [ ! -e "$SCRIPT_DIR/node_modules" ]; then
  ln -s "$WEB_DIR/node_modules" "$SCRIPT_DIR/node_modules"
fi

Xvfb ":$DISPLAY_NUM" -screen 0 "${W}x${H}x24" -nolisten tcp &
XVFB_PID=$!

MODULE_ID=$(pactl load-module module-null-sink \
  sink_name=$SINK sink_properties=device.description=chordz_capture)

FFMPEG_PID=""
cleanup() {
  [ -n "$FFMPEG_PID" ] && kill -INT "$FFMPEG_PID" 2>/dev/null && wait "$FFMPEG_PID" 2>/dev/null || true
  kill "$XVFB_PID" 2>/dev/null || true
  pactl unload-module "$MODULE_ID" 2>/dev/null || true
}
trap cleanup EXIT

# Give Xvfb and the sink a moment to come up before anything attaches to them.
sleep 1

ffmpeg -hide_banner -loglevel warning -y \
  -f x11grab -framerate "$FPS" -video_size "${W}x${H}" -i ":$DISPLAY_NUM" \
  -f pulse -i "${SINK}.monitor" \
  -c:v libx264 -preset ultrafast -crf 16 -pix_fmt yuv420p \
  -c:a flac \
  "$OUT" &
FFMPEG_PID=$!

sleep 1  # let ffmpeg open both inputs before the app makes any sound

DISPLAY=":$DISPLAY_NUM" PULSE_SINK="$SINK" node "$DRIVER"

# Flush the muxer: -INT makes ffmpeg finalize the file instead of truncating it.
kill -INT "$FFMPEG_PID"
wait "$FFMPEG_PID" 2>/dev/null || true
FFMPEG_PID=""

echo "gravado: $OUT"
