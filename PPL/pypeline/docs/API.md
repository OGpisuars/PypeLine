# PypeLine scripting API

<!-- Generated from src/engine/ui/help.rs, the in-game Help (F1). Do not edit by hand: run `PYPELINE_WRITE_DOCS=1 cargo test api_doc` to update it. -->

Everything a PypeLine script can use. The same text is in the game's Help window (F1), where each example can be inserted into `main.py`. Every example here runs: the test suite runs them all.

## How it works

Your script describes the WHOLE factory. Each time you press Run, the game builds exactly what the script says: new things are added, changed things are updated, and anything you took out of the script is removed (its items go back to the station).

Running the same script again changes nothing, so it is safe to press Run often.

If the script has an error, nothing in the factory changes and the belts stop until your next good Run. The console shows the file and line with the problem.

## The grid

The plot is 16 tiles wide and 10 tiles tall.

x goes from 0 (left) to 15 (right). y goes from 0 (bottom) to 9 (top).

Point at the island with the mouse to see a tile's x and y in the top-right corner.

Directions: "north" is up, "east" is right, "south" is down, "west" is left.

## Imports

Put these at the top of `main.py` to use every command:

```python
from auto import conveyors, machines
import power
import console
import sensors
import stats
import clock
```

| Module | What it is for |
|---|---|
| `auto.conveyors` | place belts |
| `auto.machines` | place machines; switch them on and off; read their status |
| `power` | connect machines to steam generators |
| `console` | colors and clearing for print() |
| `sensors` | how many items are on a belt tile |
| `stats` | items made, items per minute, coins |
| `clock` | the current tick and second |

## Names you can use

### Machine kinds: `machines.place("...")`

| Name | What it does |
|---|---|
| `"miner"` | digs one ore every 2 s; needs ore="iron" and power |
| `"smelter"` | turns ore into a plate every 3 s; needs power |
| `"steam_generator"` | powers machines you connect to it |
| `"station"` | holds items; the train buys them every 30 s |

### Items: `stats.produced("...")`

| Name | What it is |
|---|---|
| `"iron_ore"` | iron ore: dug by miners; the train pays 1 |
| `"iron_plate"` | iron plate: made by smelters from iron ore; the train pays 4 |

### Ores: `ore="..."`

| Name | What it does |
|---|---|
| `"iron"` | digs iron ore |

### Directions: `dir="..."`

| Name | Which way |
|---|---|
| `"north"` | up (y + 1) |
| `"east"` | right (x + 1) |
| `"south"` | down (y - 1) |
| `"west"` | left (x - 1) |

### Console colors: `console.color("...")`

`"green"` `"red"` `"yellow"` `"blue"` `"orange"` `"gray"` `"default"`

### Tiers: `tier=1`, `2` or `3`

1 is the normal part. 2 and 3 are bought in the Shop (F3).

## Building (in main.py)

### `conveyors.place(x, y, dir, tier=1)`

Puts a conveyor belt on tile (x, y).

Items on it move toward dir and are handed to whatever is on the next tile: another belt or a machine.

tier=2 and tier=3 are faster belts from the Shop.

```python
for x in range(1, 5):
    conveyors.place(x=x, y=0, dir="east")
```

### `machines.place(kind, name, x, y, dir="east", ore=..., tier=1)`

Builds a machine on tile (x, y). See "Names you can use" for the kinds.

name must be different for every machine. It is how power.connect and machines.enable find it.

dir is the side items come out of (the small brass mark). Leave it out for east.

Miners need ore="iron". tier=2 and tier=3 are faster machines from the Shop.

```python
machines.place("miner", name="miner_1", x=0, y=0, ore="iron")
machines.place("smelter", name="smelter_1", x=5, y=0)
conveyors.place(x=6, y=0, dir="east")
machines.place("station", name="station_1", x=7, y=0)
```

### `power.connect(generator, to)`

Powers machines from a steam generator. to is a list of machine names.

Place the generator and the machines first, then connect them.

A machine without power looks gray and does nothing.

```python
machines.place("steam_generator", name="steam_1", x=0, y=2)
power.connect(generator="steam_1", to=["miner_1", "smelter_1"])
```

### `console.color(name) / console.clear()`

console.color("green") colors the lines you print after it; "default" goes back.

console.clear() wipes the console, handy for dashboards that redraw.

```python
console.color("green")
print("[" + "#" * 8 + "..]")
console.color("default")
```

### `print(...)`

Writes to the console. Handy for checking what a variable holds.

```python
print("Hello, factory!")
```

## Running (in tick() and events)

### `def tick():`

If main.py defines tick(), the game calls it 20 times a second while the factory runs. Variables made outside it keep their values between ticks.

Each tick has a small step budget. A tick() that runs out of steam 3 ticks in a row overheats the boiler. Use yield inside tick() to spread work over several ticks.

Build calls (place, connect) are not allowed inside tick().

```python
def tick():
    if stats.per_minute("iron_plate") < 10:
        machines.enable("miner_1")
    else:
        machines.disable("miner_1")
```

### `machines.enable(name) / machines.disable(name)`

Switch a machine on or off. An off machine keeps its items but does no work.

Changes happen on the next factory step.

```python
def on_contract_complete(title):
    machines.enable("smelter_1")
```

### `machines.status(name)`

A dict about the machine: kind, working, input, output, powered, on, tier.

```python
def show_smelter():
    s = machines.status("smelter_1")
    print(s["kind"], "has", s["input"], "ore waiting")
```

### `sensors.count(x, y) / sensors.temperature(generator)`

How many items are on the belt at (x, y) right now (0 if there is no belt).

A steam generator's temperature. At 100 it overheats and powers nothing until it cools to 70. Each switched-on machine it powers adds 6 degrees, and the air is up to 15 degrees warmer at noon.

```python
def belt_is_busy():
    return sensors.count(2, 0) >= 2

def boiler_is_hot():
    return sensors.temperature("steam_1") >= 90
```

### `stats.produced(item) / stats.per_minute(item) / stats.coins()`

Items made since the start, items made in the last minute, and your coins.

item is an item id like "iron_plate" (see "Names you can use").

```python
def report():
    print(stats.produced("iron_ore"), "ore,", stats.coins(), "coins")
```

### `stats.bottlenecks() / stats.steam() / stats.steam_limit()`

stats.bottlenecks() lists the machines whose output is full: whatever comes after them is too slow or missing. The Stats window (F4) shows them glowing red.

stats.steam() is how much steam this Run or tick() has used so far, and stats.steam_limit() how much it has in total.

machines.status(name)["state"] is "working", "starved", "blocked", "no power", "off" or "idle".

```python
def check():
    for name in stats.bottlenecks():
        print(name, "is blocked")
    print(stats.steam(), "of", stats.steam_limit(), "steam used")
```

### `clock.tick() / clock.seconds() / clock.time_of_day() / clock.is_day()`

Ticks and whole seconds the factory has run (20 ticks a second).

The hour (0-23) and whether it is day (6:00 to 18:00). A whole day lasts four minutes; the clock in the top bar shows it.

```python
def every_ten_seconds():
    return clock.tick() % 200 == 0

def hot_hours():
    return clock.is_day() and 10 <= clock.time_of_day() <= 14
```

### `def on_train(coins): / def on_contract_complete(title):`

Events: define them and the game calls them when the cargo train pays you, or when you finish a contract.

```python
def on_train(coins):
    print("The train paid", coins, "coins")
```

## Your own files

Files > + New file makes another file next to main.py. Type a name like PPL and it is saved as PPL.py (if you type PPL.py it stays PPL.py).

A name must start with a letter and use only letters, digits and _, because main.py uses it in an import.

Run always starts main.py. Other files are modules: write import PPL in main.py, then call PPL.some_function(). Files can import each other too.

Each file has its own window. Errors name the file and the line.

## The debugger

Press Debug (F6) to record a run of main.py line by line. The Debugger window then steps through it: forward, backward, or straight to the next time a line runs.

The line about to run is marked in its code window, and the window lists your variables as they were just before it. Values that just changed are highlighted.

A Debug Run uses its own copy of Python and never changes the factory, so press it as often as you like. It records up to 2000 lines.

## The Shop and tiers

The cargo train pays coins for everything in your stations. Spend them in the Shop (F3) on faster parts. A bought upgrade does nothing until your script asks for it with tier=, so you choose where the fast parts go.

| Upgrade | What it does |
|---|---|
| `Fast belt (600 coins)` | Belts 2x as fast. Use tier=2 in conveyors.place. |
| `Express belt (2500 coins)` | Belts 4x as fast. Use tier=3 in conveyors.place. |
| `Miner Mk2 (800 coins)` | Miners work 2x as fast. Use tier=2 in machines.place("miner", ...). |
| `Miner Mk3 (3500 coins)` | Miners work 4x as fast. Use tier=3 in machines.place("miner", ...). |
| `Smelter Mk2 (1200 coins)` | Smelters work 2x as fast. Use tier=2 in machines.place("smelter", ...). |
| `Smelter Mk3 (5000 coins)` | Smelters work 4x as fast. Use tier=3 in machines.place("smelter", ...). |

Changing a tier keeps the machine's items. Tier 3 is the highest for now.

**Coming later: Prestige.** Start over with a fresh factory and keep a permanent bonus. Every prestige switches your scripts to a different programming language, and the higher you go, the trickier and fussier the language gets.

## When something goes wrong

| You see | What to do |
|---|---|
| `Error (file, line N)` | Python could not run that line. Nothing in the factory changed. Fix the line and Run again. |
| `Out of steam` | The script used too many steps, usually a loop that never ends. |
| `"except:" or "finally:" is not allowed` | Name the error you expect, like except ValueError: (or except Exception: for any ordinary error). This keeps the game's safety stop from being caught. |
| `"tier 2 belts are locked"` | Buy the upgrade in the Shop (F3) first, or leave tier= out. |
| `A machine is gray` | It has no power. Connect it with power.connect. |
| `Items stop moving` | The next tile cannot take them: nothing is there, a belt points the wrong way, or the machine is full. |
| `HALTED in the corner` | The last Run failed. The belts restart after your next good Run. |
