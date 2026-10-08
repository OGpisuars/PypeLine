# 4. Conditionals

Sometimes a script should do one thing **if** something is true, and another thing otherwise.

## if and else

```python
plates = 12
if plates >= 10:
    print("Enough for the train!")
else:
    print("Keep smelting.")
```

The line after `if` is a **condition**: a question with a yes/no answer. Like loops, the lines that belong to `if` and `else` are indented, and those lines end with a colon.

## Comparisons

| Write | Means |
|-------|-------|
| `a == b` | a equals b (two `=`!) |
| `a != b` | a is not b |
| `a < b`, `a > b` | less than, greater than |
| `a <= b`, `a >= b` | at most, at least |

## elif

`elif` ("else if") checks another condition when the first was false:

```python
x = 4
if x == 0:
    print("start of the line")
elif x == 4:
    print("smelter goes here")
else:
    print("belt")
```

## Choosing what to build in a loop

Put an `if` inside a `for` loop to build different things along one row:

```python
# snippet: Belts with a smelter in the middle
from auto import conveyors, machines
import power

machines.place("steam_generator", name="steam_1", x=0, y=2)
machines.place("miner", name="miner_1", x=0, y=0, ore="iron")
for x in range(1, 8):
    if x == 4:
        machines.place("smelter", name="smelter_1", x=x, y=0)
    else:
        conveyors.place(x=x, y=0, dir="east")
machines.place("station", name="station_1", x=8, y=0)
power.connect(generator="steam_1", to=["miner_1", "smelter_1"])
```

The belts after the smelter carry plates on to the station.
