# 5. while Loops

A `for` loop runs a set number of times. A **while loop** keeps going **as long as** its condition is true.

## Counting with while

```python
x = 1
while x < 5:
    print(x)
    x = x + 1
```

This prints 1 to 4. The last line inside the loop matters: it moves `x` closer to the end. Without it, `x < 5` would stay true forever.

## Loops that never end

A loop whose condition never becomes false is an **infinite loop**. In most programs that freezes everything. In PypeLine it burns through your steam instead: the script stops with "out of steam", the boiler overheats, and nothing in your factory changes. Fix the loop and Run again.

## Shorter steps

`x = x + 1` is so common that Python has a short form, `x += 1`. It works for other maths too: `x += 2`, `coins -= 5`.

## Building until a spot is reached

A `while` loop is handy when you know where to stop rather than how many steps it takes:

```python
# snippet: Belt until the smelter
from auto import conveyors, machines
import power

machines.place("steam_generator", name="steam_1", x=0, y=2)
machines.place("miner", name="miner_1", x=0, y=0, ore="iron")
smelter_x = 6
x = 1
while x < smelter_x:
    conveyors.place(x=x, y=0, dir="east")
    x += 1
machines.place("smelter", name="smelter_1", x=smelter_x, y=0)
power.connect(generator="steam_1", to=["miner_1", "smelter_1"])
```

Move the smelter by changing `smelter_x`, and the belt follows.
