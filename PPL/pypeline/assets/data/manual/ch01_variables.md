# 1. Variables and Calls

Welcome to the Engineering Manual, engineer! Every chapter teaches one Python idea and ends with contracts that pay coins. Finish a chapter's contracts to unlock the next one.

## Calling a function

Everything you build starts with a **function call**: a name, then round brackets with the details inside.

```python
from auto import conveyors, machines
import power

machines.place("steam_generator", name="steam_1", x=0, y=2)
machines.place("miner", name="miner_1", x=0, y=0, ore="iron")
conveyors.place(x=1, y=0, dir="east")
machines.place("smelter", name="smelter_1", x=2, y=0)
power.connect(generator="steam_1", to=["miner_1", "smelter_1"])
```

Each detail is an **argument**. `x=1` means "the argument called x is 1". Text goes in quotes, like `"east"`. Numbers do not.

## Variables

A **variable** is a name for a value. Write the name, `=`, and the value:

```python
row = 3
print(row)
```

Now `row` means 3 anywhere below that line. Variables make scripts easy to change. Here the whole production line sits on one row, so moving it means changing just one number:

```python
# snippet: One production line on a row
from auto import conveyors, machines
import power

row = 3
machines.place("steam_generator", name="steam_1", x=0, y=row + 2)
machines.place("miner", name="miner_1", x=0, y=row, ore="iron")
conveyors.place(x=1, y=row, dir="east")
conveyors.place(x=2, y=row, dir="east")
machines.place("smelter", name="smelter_1", x=3, y=row)
power.connect(generator="steam_1", to=["miner_1", "smelter_1"])
```

`row + 2` is maths: Python works it out (5) before placing the generator.

## Good to know

- Names can use letters, numbers and `_`, but cannot start with a number: `line_2` works, `2line` does not.
- `=` stores a value. It is not "equals" from maths: `count = count + 1` means "count becomes one more than it was".
- Press Run as often as you like. The factory always ends up matching your script.
