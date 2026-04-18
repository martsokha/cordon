//! Engine clocks and ECS plumbing: game clock, sim time, sim
//! speed multiplier, Uid allocator, squad-uid → entity index.
//!
//! `Time<Sim>` is a dedicated clock decoupled from
//! `Time<Virtual>` so sleep acceleration can speed up sim
//! systems without also speeding up `FixedMain` or UI
//! animation.

use std::collections::HashMap;

use bevy::prelude::*;
use cordon_core::entity::squad::Squad;
use cordon_core::primitive::{GameTime, Uid};

/// Maps stable squad uids to their current ECS entity. Maintained by
/// the spawn system and used by AI systems for the rare uid → entity
/// lookups (e.g. resolving `Goal::Protect { other }`).
#[derive(Resource, Default, Debug, Clone)]
pub struct SquadIdIndex(pub HashMap<Uid<Squad>, Entity>);

/// In-game clock. Advanced by [`tick_game_time`].
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct GameClock(pub GameTime);

/// Monotonic Uid allocator. Each call to [`UidAllocator::alloc`]
/// returns a fresh `Uid<T>` typed for the caller's marker.
#[derive(Resource, Debug, Clone)]
pub struct UidAllocator {
    next: u32,
}

impl Default for UidAllocator {
    fn default() -> Self {
        Self { next: 1 }
    }
}

impl UidAllocator {
    pub fn alloc<T: Send + Sync + 'static>(&mut self) -> Uid<T> {
        let uid = Uid::new(self.next);
        self.next += 1;
        uid
    }
}

/// Per-frame fractional accumulator for [`tick_game_time`]. Keeps
/// sub-minute progress between frames so the clock doesn't
/// discretely jump whenever a whole minute happens to align with
/// a frame boundary.
#[derive(Resource, Default, Debug)]
pub struct TimeAccumulator(pub f32);

/// How many game minutes pass per real second at 1× sim speed.
/// A game day at this rate is 12 real minutes.
const GAME_MINUTES_PER_SECOND: f32 = 2.0;

/// Marker type for simulation time. Advanced each frame by
/// [`tick_sim_time`] from virtual time scaled by [`SimSpeed`].
/// Decoupled from `Time<Virtual>` so we can accelerate the sim
/// (e.g. during sleep) without causing the `FixedMain` loop to
/// explode into dozens of physics/transform ticks per frame.
///
/// Every sim system reads `Res<Time<Sim>>` instead of `Res<Time>`
/// so it sees the scaled delta.
#[derive(Default, Debug, Clone, Copy)]
pub struct Sim;

/// Simulation speed multiplier. 1.0 = normal play. Set to e.g.
/// 50.0 during sleep to fast-forward the sim while the screen is
/// black. `Time<Virtual>` stays at 1×; only `Time<Sim>` sees the
/// speedup.
#[derive(Resource, Debug, Clone, Copy)]
pub struct SimSpeed(pub f64);

impl Default for SimSpeed {
    fn default() -> Self {
        Self(1.0)
    }
}

/// Advance `Time<Sim>` each frame from virtual time × sim speed.
pub fn tick_sim_time(
    virtual_time: Res<Time<Virtual>>,
    mut sim_time: ResMut<Time<Sim>>,
    speed: Res<SimSpeed>,
) {
    let delta = virtual_time.delta().mul_f64(speed.0);
    sim_time.advance_by(delta);
}

/// Per-frame clock advance. Reads `Time<Sim>` so the game clock
/// scales with [`SimSpeed`] instead of `Time<Virtual>`. This
/// means sleep acceleration doesn't touch virtual time and
/// doesn't disturb the `FixedMain` loop.
pub fn tick_game_time(
    time: Res<Time<Sim>>,
    mut acc: ResMut<TimeAccumulator>,
    mut clock: ResMut<GameClock>,
) {
    acc.0 += time.delta_secs() * GAME_MINUTES_PER_SECOND;
    let minutes = acc.0 as u32;
    if minutes > 0 {
        acc.0 -= minutes as f32;
        clock.0.advance_minutes(minutes);
    }
}
