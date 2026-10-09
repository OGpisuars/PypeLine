from auto import conveyors, machines
import power

# Three production lines from one loop
machines.place("steam_generator", name="steam", x=8, y=9)
powered = []
for n, y in enumerate([1, 4, 7]):
    machines.place("miner", name=f"miner_{n}", x=0, y=y, ore="iron")
    for x in range(1, 6):
        conveyors.place(x=x, y=y, dir="east")
    machines.place("smelter", name=f"smelter_{n}", x=6, y=y)
    for x in range(7, 12):
        conveyors.place(x=x, y=y, dir="east")
    machines.place("station", name=f"station_{n}", x=12, y=y)
    powered += [f"miner_{n}", f"smelter_{n}"]

power.connect(generator="steam", to=powered)
