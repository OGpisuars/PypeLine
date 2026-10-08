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
cat > "$data/PypeLine/scripts/main.py" <<'PY'
from auto import conveyors, machines
import power

# Three production lines from one loop
machines.place("steam_generator", name="steam", x=8, y=9)
powered = []
for n, y in enumerate([1, 4, 7]):
    machines.place("miner", name=f"miner_{n}", x=0, y=y, ore="iron")
    for x in range(1, 6):
        conveyors.place(x=x, y=y, dir="east")
    machines.place("smelter", name=f"smelter_{n}", x=6, y=y)
    for x in range(7, 12):
        conveyors.place(x=x, y=y, dir="east")
    machines.place("station", name=f"station_{n}", x=12, y=y)
    powered += [f"miner_{n}", f"smelter_{n}"]

power.connect(generator="steam", to=powered)
PY

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
