# 10. tick() and Generators

Events run when something happens. Sometimes you want code that runs **all the time**, watching the factory and adjusting it. That is `tick()`.

## tick(): your code, 20 times a second

If your script defines a function called `tick`, the game calls it on every game tick, 20 times a second, for as long as the factory runs:

```python
import clock

def tick():
    if clock.tick() % 200 == 0:
        print(f"{clock.seconds()} seconds and counting")
```

`clock.tick()` is how many ticks the factory has run, and `clock.seconds()` the same in seconds. `% 200 == 0` is true once every 200 ticks, which is every 10 seconds.

Three rules keep `tick()` fair:

- It gets a small **step budget** each tick (much less than a whole Run). Run out three ticks in a row and the boiler overheats.
- It cannot build. `place` and `connect` belong in `main.py`; `tick()` reads sensors and switches machines.
- If it has an error, the game switches `tick()` off until your next Run. The factory keeps going.

## A thermostat for your belts

Here is a classic job for `tick()`: stop a miner when the belt in front of the smelter is backing up, and start it again when the belt clears.

```python
# snippet: Belt thermostat
from auto import machines
import sensors

def tick():
    if sensors.count(1, 0) >= 2:
        machines.disable("miner_1")
    else:
        machines.enable("miner_1")
```

## Too much work for one tick

What if you have twenty lines to check? Doing all of them every tick could run out of steam. A **generator** spreads the work out.

Put `yield` inside a function and it becomes a generator: at each `yield` it **pauses**, and the next time it is called it carries on from that exact spot. When `tick()` is a generator, every game tick runs it up to the next `yield`:

```python
from auto import machines
import sensors

rows = [0, 3, 6]

def tick():
    for y in rows:
        if sensors.count(1, y) >= 2:
            machines.disable(f"miner_{y}")
        else:
            machines.enable(f"miner_{y}")
        yield
```

The first tick checks row 0, the next tick row 3, then row 6. When the loop ends, the generator starts over. Each tick does a third of the work, so even a huge factory stays inside the budget.

## Generators outside tick()

`yield` works in any function. Looping over a generator gives you each value it yields:

```python
def countdown(n):
    while n > 0:
        yield n
        n = n - 1

for number in countdown(3):
    print(number)
print("Lift off!")
```
