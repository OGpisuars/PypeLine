//! The game modules Python scripts can import: `auto` and `power`.
//!
//! Each call checks its arguments right away and records into the run's
//! `BuildPlan`, so a mistake raises a Python error on the exact line.
//! Fresh module objects are built for every run, so a script that pokes at a
//! module (`auto.conveyors.place = None`) cannot break the next run.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use rustpython_vm::{
    AsObject, FromArgs, PyObjectRef, PyResult, VirtualMachine, builtins::PyStrRef,
    function::ArgSequence,
};

use super::commands::{self, BuildPlan, PlannedBelt, PlannedMachine};
use super::console_api::{ConsoleSink, console_module};
use super::operate::{ApiMode, Op, WorldView};
use crate::factory::items;
use crate::factory::items::ItemKind;
use crate::factory::machines::MachineKind;
use crate::factory::{Dir, Pos};

/// Importable modules for one run, by their full dotted name.
pub type ModuleTable = BTreeMap<String, PyObjectRef>;

#[derive(FromArgs)]
struct ConveyorArgs {
    #[pyarg(any)]
    x: i64,
    #[pyarg(any)]
    y: i64,
    #[pyarg(any)]
    dir: PyStrRef,
    #[pyarg(any, optional)]
    tier: Option<i64>,
}

#[derive(FromArgs)]
struct MachineArgs {
    #[pyarg(positional)]
    kind: PyStrRef,
    #[pyarg(any)]
    name: PyStrRef,
    #[pyarg(any)]
    x: i64,
    #[pyarg(any)]
    y: i64,
    #[pyarg(any, optional)]
    dir: Option<PyStrRef>,
    #[pyarg(any, optional)]
    ore: Option<PyStrRef>,
    #[pyarg(any, optional)]
    tier: Option<i64>,
}

#[derive(FromArgs)]
struct ConnectArgs {
    #[pyarg(any)]
    generator: PyStrRef,
    #[pyarg(any)]
    to: ArgSequence<PyStrRef>,
}

/// Everything the game modules share during a run or a tick.
#[derive(Clone, Default)]
pub struct ScriptContext {
    pub plan: Rc<RefCell<BuildPlan>>,
    pub console: Rc<RefCell<ConsoleSink>>,
    pub mode: Rc<Cell<ApiMode>>,
    pub world: Rc<RefCell<WorldView>>,
    pub ops: Rc<RefCell<Vec<Op>>>,
}

const BUILD_ONLY: &str =
    "build calls (place and connect) go in main.py, not in tick() or an event handler";
const OPERATE_ONLY: &str = "machines.enable and machines.disable work inside tick() or an event handler, not while building";

/// Build this run's modules, all sharing `ctx`.
pub fn build_modules(vm: &VirtualMachine, ctx: &ScriptContext) -> PyResult<ModuleTable> {
    let plan = &ctx.plan;
    let console = &ctx.console;
    let place_conveyor = {
        let plan = plan.clone();
        let mode = ctx.mode.clone();
        let world = ctx.world.clone();
        vm.new_function(
            "place",
            move |args: ConveyorArgs, vm: &VirtualMachine| -> PyResult<()> {
                if mode.get() != ApiMode::Build {
                    return Err(vm.new_value_error(BUILD_ONLY));
                }
                let pos = pos(args.x, args.y, vm)?;
                let belt = PlannedBelt {
                    dir: dir(&args.dir, vm)?,
                    tier: commands::belt_tier(args.tier.unwrap_or(1), &world.borrow().unlocked)
                        .map_err(|msg| vm.new_value_error(msg))?,
                };
                plan.borrow_mut()
                    .place_conveyor(pos, belt)
                    .map_err(|msg| vm.new_value_error(msg))
            },
        )
    };

    let place_machine = {
        let plan = plan.clone();
        let mode = ctx.mode.clone();
        let world = ctx.world.clone();
        vm.new_function(
            "place",
            move |args: MachineArgs, vm: &VirtualMachine| -> PyResult<()> {
                if mode.get() != ApiMode::Build {
                    return Err(vm.new_value_error(BUILD_ONLY));
                }
                let kind_name = text(&args.kind, vm)?;
                let kind = MachineKind::by_name(&kind_name).ok_or_else(|| {
                    let known: Vec<_> = MachineKind::ALL
                        .iter()
                        .map(|k| format!("\"{}\"", k.name()))
                        .collect();
                    vm.new_value_error(format!(
                        "unknown machine \"{kind_name}\" (try {})",
                        known.join(", ")
                    ))
                })?;
                let ore = match &args.ore {
                    Some(ore) => {
                        let ore = text(ore, vm)?;
                        Some(items::ore_by_name(&ore).ok_or_else(|| {
                            vm.new_value_error(format!("unknown ore \"{ore}\" (try \"iron\")"))
                        })?)
                    }
                    None => None,
                };
                let tier =
                    commands::machine_tier(kind, args.tier.unwrap_or(1), &world.borrow().unlocked)
                        .map_err(|msg| vm.new_value_error(msg))?;
                let machine = PlannedMachine {
                    kind,
                    pos: pos(args.x, args.y, vm)?,
                    dir: match &args.dir {
                        Some(d) => dir(d, vm)?,
                        None => Dir::East,
                    },
                    ore,
                    tier,
                };
                plan.borrow_mut()
                    .place_machine(&text(&args.name, vm)?, machine)
                    .map_err(|msg| vm.new_value_error(msg))
            },
        )
    };

    let connect = {
        let plan = plan.clone();
        let mode = ctx.mode.clone();
        vm.new_function(
            "connect",
            move |args: ConnectArgs, vm: &VirtualMachine| -> PyResult<()> {
                if mode.get() != ApiMode::Build {
                    return Err(vm.new_value_error(BUILD_ONLY));
                }
                let generator = text(&args.generator, vm)?;
                let machines = args
                    .to
                    .as_slice()
                    .iter()
                    .map(|name| text(name, vm))
                    .collect::<PyResult<Vec<_>>>()?;
                plan.borrow_mut()
                    .connect(&generator, &machines)
                    .map_err(|msg| vm.new_value_error(msg))
            },
        )
    };

    let conveyors = new_module(vm, "auto.conveyors", &[("place", place_conveyor.into())])?;
    let operate = operate_functions(vm, ctx);
    let machines = new_module(
        vm,
        "auto.machines",
        &[
            ("place", place_machine.into()),
            ("enable", operate.enable),
            ("disable", operate.disable),
            ("status", operate.status),
        ],
    )?;
    let auto = new_module(
        vm,
        "auto",
        &[
            ("conveyors", conveyors.clone()),
            ("machines", machines.clone()),
        ],
    )?;
    let power = new_module(vm, "power", &[("connect", connect.into())])?;
    let console = new_module(vm, "console", &console_module(vm, console)?)?;
    let sensors = new_module(vm, "sensors", &[("count", operate.count)])?;
    let stats = new_module(
        vm,
        "stats",
        &[
            ("produced", operate.produced),
            ("per_minute", operate.per_minute),
            ("coins", operate.coins),
        ],
    )?;
    let clock = new_module(
        vm,
        "clock",
        &[("tick", operate.tick), ("seconds", operate.seconds)],
    )?;

    Ok(BTreeMap::from([
        ("auto".to_owned(), auto),
        ("auto.conveyors".to_owned(), conveyors),
        ("auto.machines".to_owned(), machines),
        ("power".to_owned(), power),
        ("console".to_owned(), console),
        ("sensors".to_owned(), sensors),
        ("stats".to_owned(), stats),
        ("clock".to_owned(), clock),
    ]))
}

fn new_module(
    vm: &VirtualMachine,
    name: &str,
    attrs: &[(&'static str, PyObjectRef)],
) -> PyResult<PyObjectRef> {
    let module = vm.new_module(name, vm.ctx.new_dict(), None);
    for (attr, value) in attrs {
        module.as_object().set_attr(*attr, value.clone(), vm)?;
    }
    Ok(module.into())
}

fn text(s: &PyStrRef, vm: &VirtualMachine) -> PyResult<String> {
    s.to_str()
        .map(str::to_owned)
        .ok_or_else(|| vm.new_value_error("text must be plain letters and numbers"))
}

fn pos(x: i64, y: i64, vm: &VirtualMachine) -> PyResult<Pos> {
    let x =
        i32::try_from(x).map_err(|_| vm.new_value_error(format!("x={x} is far off the plot")))?;
    let y =
        i32::try_from(y).map_err(|_| vm.new_value_error(format!("y={y} is far off the plot")))?;
    Ok(Pos::new(x, y))
}

fn dir(name: &PyStrRef, vm: &VirtualMachine) -> PyResult<Dir> {
    let name = text(name, vm)?;
    Dir::by_name(&name).ok_or_else(|| {
        vm.new_value_error(format!(
            "unknown direction \"{name}\" (use \"north\", \"east\", \"south\" or \"west\")"
        ))
    })
}

struct OperateFunctions {
    enable: PyObjectRef,
    disable: PyObjectRef,
    status: PyObjectRef,
    count: PyObjectRef,
    produced: PyObjectRef,
    per_minute: PyObjectRef,
    coins: PyObjectRef,
    tick: PyObjectRef,
    seconds: PyObjectRef,
}

/// Functions that read the world snapshot or queue operations.
fn operate_functions(vm: &VirtualMachine, ctx: &ScriptContext) -> OperateFunctions {
    let switch = |on: bool| {
        let (mode, world, ops) = (ctx.mode.clone(), ctx.world.clone(), ctx.ops.clone());
        let name = if on { "enable" } else { "disable" };
        vm.new_function(
            name,
            move |machine: PyStrRef, vm: &VirtualMachine| -> PyResult<()> {
                if mode.get() != ApiMode::Operate {
                    return Err(vm.new_value_error(OPERATE_ONLY));
                }
                let machine = text(&machine, vm)?;
                if !world.borrow().machines.contains_key(&machine) {
                    return Err(
                        vm.new_value_error(format!("there is no machine named '{machine}'"))
                    );
                }
                ops.borrow_mut().push(Op::SetEnabled { machine, on });
                Ok(())
            },
        )
        .into()
    };
    let status = {
        let world = ctx.world.clone();
        vm.new_function(
            "status",
            move |machine: PyStrRef, vm: &VirtualMachine| -> PyResult {
                let name = text(&machine, vm)?;
                let world = world.borrow();
                let m = world.machines.get(&name).ok_or_else(|| {
                    vm.new_value_error(format!("there is no machine named '{name}'"))
                })?;
                let dict = vm.ctx.new_dict();
                dict.set_item("kind", vm.ctx.new_str(m.kind.name()).into(), vm)?;
                dict.set_item("working", vm.ctx.new_bool(m.working).into(), vm)?;
                dict.set_item("input", vm.ctx.new_int(m.input).into(), vm)?;
                dict.set_item("output", vm.ctx.new_int(m.output).into(), vm)?;
                dict.set_item("powered", vm.ctx.new_bool(m.powered).into(), vm)?;
                dict.set_item("on", vm.ctx.new_bool(m.enabled).into(), vm)?;
                dict.set_item("tier", vm.ctx.new_int(m.tier).into(), vm)?;
                Ok(dict.into())
            },
        )
        .into()
    };
    let count = {
        let world = ctx.world.clone();
        vm.new_function(
            "count",
            move |x: i64, y: i64, vm: &VirtualMachine| -> PyResult<usize> {
                let at = pos(x, y, vm)?;
                Ok(world.borrow().belts.get(&at).copied().unwrap_or(0))
            },
        )
        .into()
    };
    let item_stat = |name: &'static str, per_minute: bool| {
        let world = ctx.world.clone();
        vm.new_function(
            name,
            move |item: PyStrRef, vm: &VirtualMachine| -> PyResult<u64> {
                let id = text(&item, vm)?;
                let item = ItemKind::by_id(&id).ok_or_else(|| {
                    let ids: Vec<String> = ItemKind::ALL
                        .iter()
                        .map(|i| format!("\"{}\"", i.id()))
                        .collect();
                    vm.new_value_error(format!("unknown item \"{id}\" (try {})", ids.join(" or ")))
                })?;
                let world = world.borrow();
                let table = if per_minute {
                    &world.per_minute
                } else {
                    &world.produced
                };
                Ok(table.get(&item).copied().unwrap_or(0))
            },
        )
        .into()
    };
    let world_number = |name: &'static str, read: fn(&WorldView) -> u64| {
        let world = ctx.world.clone();
        vm.new_function(name, move || -> u64 { read(&world.borrow()) })
            .into()
    };
    OperateFunctions {
        enable: switch(true),
        disable: switch(false),
        status,
        count,
        produced: item_stat("produced", false),
        per_minute: item_stat("per_minute", true),
        coins: world_number("coins", |w| w.coins),
        tick: world_number("tick", |w| w.ticks),
        seconds: world_number("seconds", |w| w.ticks / 20),
    }
}
