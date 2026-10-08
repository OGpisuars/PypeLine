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

## Day, night and hot boilers

The island has days and nights. One day lasts four minutes of game time; the clock in the top bar shows the hour, and hovering over it tells you when day or night begins.

A steam generator warms up for every switched-on machine it powers, and the air is hotter by day, warmest at noon. A generator that reaches **100 degrees overheats**: it stops powering anything until it has cooled to 70. Small generators never get that hot, but one powering a dozen machines will, around midday.

| Call | What it tells you |
|------|-------------------|
| `clock.time_of_day()` | The hour, 0 to 23 |
| `clock.is_day()` | `True` from 6:00 to 18:00 |
| `sensors.temperature(name)` | A steam generator's temperature in degrees |

`tick()` can keep a big generator safe by switching a few machines off when it gets hot, and back on when it has cooled. Using two numbers (off above 90, on below 80) stops it flicking on and off every tick:

```python
# snippet: Boiler guard
from auto import machines
import sensors

def tick():
    heat = sensors.temperature("steam")
    if heat >= 90:
        machines.disable("miner_a")
        machines.disable("miner_b")
    elif heat <= 80:
        machines.enable("miner_a")
        machines.enable("miner_b")
```
