# 9. Events and Sensors

So far your script runs once, builds the factory, and is done. But a factory keeps going after that: trains come and go, machines fill up, contracts finish. **Events** let your code react while the factory runs.

## Events: functions the game calls

An **event handler** is a function with a special name. You never call it yourself. You define it, press Run, and the game calls it when the thing happens:

```python
def on_train(coins):
    print(f"The train paid {coins} coins")
```

| Event | When the game calls it |
|-------|------------------------|
| `on_train(coins)` | Every time the cargo train pays you, with what it paid |
| `on_contract_complete(title)` | When you finish a contract, with its title |

Variables outside the function keep their values between calls. To change one from inside a function, say `global` first:

```python
trains = 0
total = 0

def on_train(coins):
    global trains, total
    trains = trains + 1
    total = total + coins
    print(f"Train {trains}: +{coins} coins ({total} so far)")
```

## Your own celebration

If you define `on_contract_complete`, it replaces the game's banner. It is a good place for an ASCII banner:

```python
# snippet: Contract banner
import console

def on_contract_complete(title):
    console.color("green")
    print("+" + "-" * (len(title) + 4) + "+")
    print(f"|  {title}  |")
    print("+" + "-" * (len(title) + 4) + "+")
    console.color("default")
```

## Sensors: reading the factory

Inside an event (and in `tick()`, next chapter) your code can look at the running factory:

| Call | What it tells you |
|------|-------------------|
| `sensors.count(x, y)` | How many items are on the belt at (x, y) |
| `machines.status(name)` | A dict: `kind`, `working`, `input`, `output`, `powered`, `on`, `tier` |
| `stats.produced(item)` | How many of an item you made so far, like `"iron_plate"` |
| `stats.per_minute(item)` | How many you made in the last minute |
| `stats.coins()` | Your coins right now |

```python
import sensors
import stats

def on_train(coins):
    print(f"{stats.per_minute('iron_plate')} plates a minute")
    if sensors.count(1, 0) >= 2:
        print("The belt at (1, 0) is backing up!")
```

`machines.status` gives back a dict, so you read it with square brackets:

```python
from auto import machines

def report(name):
    s = machines.status(name)
    if not s["powered"]:
        print(f"{name} has no power!")
    elif s["output"] >= 10:
        print(f"{name} is full: nothing takes its output")
    else:
        print(f"{name} is fine")
```

## Switching machines on and off

`machines.disable(name)` stops a machine (it keeps its items) and `machines.enable(name)` starts it again. They only work inside events and `tick()`: building happens in `main.py`, running happens in handlers.

```python
from auto import machines
import stats

def on_train(coins):
    if stats.coins() > 500:
        machines.disable("miner_1")
    else:
        machines.enable("miner_1")
```
