# 8. Imports and Modules

Your factory scripts are getting long. Real programmers split big programs into several files, called **modules**, and use `import` to bring them together. You have done this since chapter 1: `auto`, `power` and `console` are modules the game gives you. Now you write your own.

## Making a file

Open **Files > + New file** in the top bar and type a name, like `lines`. The game saves it as `lines.py` (type `lines.py` and it stays `lines.py`). Every file gets its own window, so you can keep `main.py` and `lines.py` side by side.

A file name must start with a letter and use only letters, digits and `_`, because `main.py` uses that name in an `import`.

**Run always starts `main.py`.** Other files only run when something imports them.

## A module of your own

Here is a whole file. Press **Create lines.py** under it to add it to your workspace:

```python
# file: lines.py
from auto import conveyors, machines

def smelter_line(name, y):
    machines.place("miner", name=f"miner_{name}", x=0, y=y, ore="iron")
    conveyors.place(x=1, y=y, dir="east")
    machines.place("smelter", name=f"smelter_{name}", x=2, y=y)
    conveyors.place(x=3, y=y, dir="east")
    machines.place("station", name=f"station_{name}", x=4, y=y)
    return [f"miner_{name}", f"smelter_{name}"]
```

Now `main.py` can import it and call its function with a dot, just like `power.connect`:

```python
# snippet: Lines from a module
from auto import machines
import power
import lines

machines.place("steam_generator", name="steam", x=15, y=9)
powered = lines.smelter_line("a", 0) + lines.smelter_line("b", 3)
power.connect(generator="steam", to=powered)
```

The function returns the names of the machines it built, so `main.py` can power them. Adding two lists with `+` joins them into one.

## Importing just the names you need

`from lines import smelter_line` brings in the function itself, so you can call it without `lines.` in front:

```python
from auto import machines
import power
from lines import smelter_line

machines.place("steam_generator", name="steam", x=15, y=9)
powered = []
for i, y in enumerate([0, 3, 6]):
    powered = powered + smelter_line(f"line_{i}", y)
power.connect(generator="steam", to=powered)
```

`enumerate` hands you a counter next to each item: `0, 1, 2` next to the rows `0, 3, 6`.

## Modules can hold data too

A module is just Python, so it can keep values as well as functions. A file that only describes the factory is called a **plan**:

```python
# file: plan.py
ROWS = {"north": 6, "middle": 3, "south": 0}
GENERATOR = "steam"
```

Names in CAPITALS are a Python habit for values that never change. `main.py` reads them like anything else in a module:

```python
from auto import machines
import power
import lines
import plan

machines.place("steam_generator", name=plan.GENERATOR, x=15, y=9)
powered = []
for name, y in plan.ROWS.items():
    powered = powered + lines.smelter_line(name, y)
power.connect(generator=plan.GENERATOR, to=powered)
```

Now changing the factory means editing `plan.py`, and the building code never has to change.

## When something goes wrong

Errors name the file and the line, like `Error (lines.py, line 4)`, and the window of that file shows the problem line. If a module imports a module that imports it back, Python cannot finish either one, and the game tells you they go in a circle.
