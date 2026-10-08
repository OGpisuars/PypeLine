//! Fixed 20 Hz simulation loop, decoupled from rendering, plus the Time
//! Dials (pause, play, 1x/2x/4x, step one tick).
//!
//! All gameplay logic runs in `FixedUpdate`, inside the `SimSet` sets below, in
//! a fixed order. Rendering and UI run in `Update` and only read sim state.

use bevy::prelude::*;

pub const TICKS_PER_SECOND: f64 = 20.0;

/// Number of simulation ticks since the game started.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SimTick(pub u64);

/// Explicit, ordered phases of one simulation tick (roadmap Part 4 C:
/// fixed system ordering for determinism).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SimSet {
    /// Advance the tick counter.
    Clock,
    /// Run player scripts.
    Scripts,
    /// Apply queued script commands and step the factory.
    Factory,
}

/// Speeds the Time Dials offer.
pub const SPEEDS: [u32; 3] = [1, 2, 4];

/// The Time Dials. Pausing stops ticks entirely, so a Run while paused takes
/// effect on the next tick you step to (roadmap: TIME DIALS).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimControl {
    pub paused: bool,
    /// 1, 2 or 4: ticks per 1/20 s.
    pub speed: u32,
    /// Run exactly one tick while paused.
    pub step_requested: bool,
}

impl Default for SimControl {
    fn default() -> Self {
        Self {
            paused: false,
            speed: 1,
            step_requested: false,
        }
    }
}

pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_hz(TICKS_PER_SECOND))
            .init_resource::<SimTick>()
            .init_resource::<SimControl>()
            .configure_sets(
                FixedUpdate,
                (SimSet::Clock, SimSet::Scripts, SimSet::Factory)
                    .chain()
                    .run_if(should_tick),
            )
            .add_systems(Update, apply_speed)
            .add_systems(FixedUpdate, advance_tick.in_set(SimSet::Clock))
            .add_systems(FixedUpdate, consume_step.after(SimSet::Factory));
    }
}

fn should_tick(control: Res<SimControl>) -> bool {
    !control.paused || control.step_requested
}

fn consume_step(mut control: ResMut<SimControl>) {
    if control.step_requested {
        control.step_requested = false;
    }
}

/// Faster speeds run more fixed ticks per real second; rendering is unaffected.
fn apply_speed(control: Res<SimControl>, mut time: ResMut<Time<Virtual>>) {
    if control.is_changed() {
        time.set_relative_speed(control.speed as f32);
    }
}

fn advance_tick(mut tick: ResMut<SimTick>) {
    tick.0 += 1;
}
