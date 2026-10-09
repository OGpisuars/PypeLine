#!/usr/bin/env bash
# Takes the README screenshots in docs/images with the game's dev hooks
# (see src/dev.rs). Uses a throwaway profile, so your own scripts, saves and
# settings are never touched.
#
#   tools/readme_screenshots.sh          (from PPL/pypeline)
#   JOBS=2 tools/readme_screenshots.sh   (gentler build on slow machines)
set -euo pipefail
cd "$(dirname "$0")/.."

cargo build -j "${JOBS:-4}"
game=target/debug/pypeline
out=docs/images
mkdir -p "$out"

data=$(mktemp -d)
trap 'rm -rf "$data"' EXIT
mkdir -p "$data/PypeLine/scripts"
cp tools/readme_main.py "$data/PypeLine/scripts/main.py"

# shot NAME SECONDS [VAR=value ...]: run the game, screenshot after SECONDS.
shot() {
    local name=$1 after=$2
    shift 2
    echo "taking $out/$name.png"
    env XDG_DATA_HOME="$data" PYPELINE_SCREENSHOT="$out/$name.png" \
        PYPELINE_SCREENSHOT_AFTER="$after" "$@" "$game" > /dev/null 2>&1
}

shot logo 3.4 PYPELINE_SCREEN=logo
shot title 3 PYPELINE_SCREEN=menu
shot factory 35 PYPELINE_AUTORUN=1 PYPELINE_OPEN=-manual
shot stats 35 PYPELINE_AUTORUN=1 PYPELINE_OPEN=-manual,stats
shot manual 6 PYPELINE_AUTORUN=1 PYPELINE_OPEN=manual
echo "done: $(ls "$out"/*.png | wc -l) screenshots in $out"
