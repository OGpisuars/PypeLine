#!/usr/bin/env bash
# Records the README GIF (docs/images/demo.gif): the code window is open on
# an empty island, Run is pressed, and the factory snaps into place and gets
# going. Uses a throwaway profile like readme_screenshots.sh. Needs ffmpeg.
#
#   tools/readme_gif.sh          (from PPL/pypeline)
#   JOBS=2 tools/readme_gif.sh   (gentler build on slow machines)
set -euo pipefail
cd "$(dirname "$0")/.."
command -v ffmpeg > /dev/null || { echo "needs ffmpeg"; exit 1; }

cargo build -j "${JOBS:-4}"
game=target/debug/pypeline
out=docs/images/demo.gif

data=$(mktemp -d)
frames=$(mktemp -d)
trap 'rm -rf "$data" "$frames"' EXIT
mkdir -p "$data/PypeLine/scripts"
cp tools/readme_main.py "$data/PypeLine/scripts/main.py"

echo "recording frames (about 10 seconds)"
env XDG_DATA_HOME="$data" PYPELINE_SCREEN=play PYPELINE_OPEN=-manual \
    PYPELINE_AUTORUN_AFTER=2.5 PYPELINE_RECORD="$frames" \
    PYPELINE_RECORD_FPS=12 PYPELINE_RECORD_SECONDS=8 \
    "$game" > /dev/null 2>&1

# Landscape crop from the top (tiling window managers can make the window
# tall), 960 px wide, with a palette made for these frames.
filters="crop=iw:'min(ih,iw*0.72)':0:0,scale=960:-1:flags=lanczos"
ffmpeg -loglevel error -y -framerate 12 -i "$frames/frame_%05d.png" \
    -vf "$filters,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=none" \
    -loop 0 "$out"
echo "done: $out ($(du -h "$out" | cut -f1))"
