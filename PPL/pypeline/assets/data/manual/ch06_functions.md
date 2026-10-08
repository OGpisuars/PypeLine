# 6. Functions

You have been **calling** functions since chapter 1. Now you will **write** your own. A function is a recipe with a name: define it once, use it as often as you like.

## def

```python
def greet(name):
    print(f"Hello, {name}!")

greet("engineer")
greet("station master")
```

`def` starts the definition. `name` is a **parameter**: a variable that gets its value from each call. The indented lines are the function's body. Nothing happens until you call it.

## Returning a value

`return` hands a result back to whoever called the function:

```python
def plates_for(coins):
    return coins // 4

print(plates_for(40))
```

`//` divides and drops the remainder: 40 coins buy 10 plates.

## A production-line kit

Wrap a whole line in a function, and building a new one is a single call:

```python
# snippet: build_line function
from auto import conveyors, machines
import power

machines.place("steam_generator", name="steam", x=15, y=9)

def build_line(y, number):
    miner = f"miner_{number}"
    smelter = f"smelter_{number}"
    machines.place("miner", name=miner, x=0, y=y, ore="iron")
    conveyors.place(x=1, y=y, dir="east")
    machines.place("smelter", name=smelter, x=2, y=y)
    conveyors.place(x=3, y=y, dir="east")
    machines.place("station", name=f"station_{number}", x=4, y=y)
    power.connect(generator="steam", to=[miner, smelter])

build_line(0, 1)
build_line(3, 2)
```

Variables made inside a function (like `miner` above) only exist inside it. Each call gets fresh ones.
