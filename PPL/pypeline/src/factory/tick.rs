//! Fixed 20 Hz simulation loop, decoupled from rendering.
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

pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_hz(TICKS_PER_SECOND))
            .init_resource::<SimTick>()
            .configure_sets(
                FixedUpdate,
                (SimSet::Clock, SimSet::Scripts, SimSet::Factory).chain(),
            )
            .add_systems(FixedUpdate, advance_tick.in_set(SimSet::Clock));
    }
}

fn advance_tick(mut tick: ResMut<SimTick>) {
    tick.0 += 1;
}
