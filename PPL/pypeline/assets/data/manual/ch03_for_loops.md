# 3. for Loops

Placing belts one by one gets old fast. A **for loop** runs the same lines once for every value in a range.

## range

```python
for x in range(1, 5):
    print(x)
```

This prints 1, 2, 3 and 4. `range(1, 5)` starts at 1 and stops **before** 5. The lines that repeat are the ones **indented** under the `for` line (4 spaces), and the `for` line ends with a colon `:`.

## Laying belts with a loop

```python
# snippet: A long belt with a for loop
from auto import conveyors, machines
import power

machines.place("steam_generator", name="steam_1", x=0, y=2)
machines.place("miner", name="miner_1", x=0, y=0, ore="iron")
for x in range(1, 9):
    conveyors.place(x=x, y=0, dir="east")
machines.place("smelter", name="smelter_1", x=9, y=0)
power.connect(generator="steam_1", to=["miner_1", "smelter_1"])
```

Want a longer line? Change one number.

## Loops with f-strings

The loop variable works anywhere, including inside f-strings:

```python
for n in range(1, 4):
    print(f"Building line {n}")
```

## Several lines at once

A loop can build whole production lines. Each one needs its own names, so the names use the loop variable:

```python
# snippet: Three lines from one loop
from auto import conveyors, machines
import power

machines.place("steam_generator", name="steam_1", x=0, y=9)
for n in range(3):
    y = n * 3
    machines.place("miner", name=f"miner_{n}", x=0, y=y, ore="iron")
    conveyors.place(x=1, y=y, dir="east")
    machines.place("smelter", name=f"smelter_{n}", x=2, y=y)
    power.connect(generator="steam_1", to=[f"miner_{n}", f"smelter_{n}"])
```

`range(3)` with one number counts 0, 1, 2.
