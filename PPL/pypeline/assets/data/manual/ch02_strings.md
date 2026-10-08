# 2. Strings and f-strings

A **string** is text in quotes. You have been writing them all along: `"miner"`, `"east"`, `"steam_1"`. This chapter shows what else strings can do, and how to make the console tell you things.

## Printing

`print` writes to the console under your code:

```python
print("Boiler lit!")
print("Plates per minute:", 20)
```

## Gluing strings together

`+` joins strings, and `*` repeats one:

```python
name = "miner" + "_" + "1"
print(name)
print("=" * 20)
```

## f-strings

Put an `f` before the quote and you can drop values straight into the text with `{ }`:

```python
plates = 12
coins = plates * 4
print(f"Made {plates} plates, worth {coins} coins")
```

f-strings are perfect for names that follow a pattern:

```python
number = 2
print(f"miner_{number}")
```

## Selling to the train

A **station** collects items from a belt pointing into it. Every 30 seconds the cargo train buys everything inside: 1 coin per ore, 4 coins per plate. Point a belt from your smelter into a station:

```python
# snippet: Smelter into a station
from auto import conveyors, machines
import power

machines.place("steam_generator", name="steam_1", x=0, y=2)
machines.place("miner", name="miner_1", x=0, y=0, ore="iron")
conveyors.place(x=1, y=0, dir="east")
machines.place("smelter", name="smelter_1", x=2, y=0)
conveyors.place(x=3, y=0, dir="east")
machines.place("station", name="station_1", x=4, y=0)
power.connect(generator="steam_1", to=["miner_1", "smelter_1"])
print(f"Ready to sell from {4}, {0}")
```

## A progress bar

Strings and `*` make simple pictures in the console:

```python
done = 7
total = 10
print("[" + "#" * done + "." * (total - done) + "]")
```
