# 7. Lists and Dicts

So far each variable held one value. **Lists** and **dicts** hold many.

## Lists

A list is values in square brackets, in order:

```python
rows = [0, 3, 6]
print(rows[0])
print(len(rows))
```

`rows[0]` is the first item (counting starts at 0) and `len(rows)` is how many there are. You have already used a list: `to=["miner_1", "smelter_1"]` in `power.connect`.

A `for` loop can walk straight through a list:

```python
for y in [0, 3, 6]:
    print(f"line on row {y}")
```

Lists can grow with `append`:

```python
machines_to_power = []
machines_to_power.append("miner_1")
machines_to_power.append("smelter_1")
print(machines_to_power)
```

## Dicts

A **dict** maps keys to values, like a little phone book:

```python
prices = {"iron ore": 1, "iron plate": 4}
print(prices["iron plate"])
```

Loop over a dict's pairs with `.items()`:

```python
prices = {"iron ore": 1, "iron plate": 4}
for item, price in prices.items():
    print(f"{item}: {price} coins")
```

## A factory plan as data

Describe the factory as data, then let one loop build it:

```python
# snippet: Build lines from a dict
from auto import conveyors, machines
import power

machines.place("steam_generator", name="steam", x=15, y=9)
lines = {"north": 6, "middle": 3, "south": 0}
powered = []
for name, y in lines.items():
    machines.place("miner", name=f"miner_{name}", x=0, y=y, ore="iron")
    conveyors.place(x=1, y=y, dir="east")
    machines.place("smelter", name=f"smelter_{name}", x=2, y=y)
    conveyors.place(x=3, y=y, dir="east")
    machines.place("station", name=f"station_{name}", x=4, y=y)
    powered.append(f"miner_{name}")
    powered.append(f"smelter_{name}")
power.connect(generator="steam", to=powered)
```

Adding a line is now just one more entry in `lines`.
