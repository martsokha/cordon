//! Area waypoint rolling shared between spawn-time generation and
//! player-issued commands.
//!
//! Both code paths (NPC squad generator, `SquadCommand::Patrol|Scavenge`)
//! need to roll a small ring of scattered waypoints inside a given
//! area so squads actually move when told to patrol. The math is
//! identical at both sites; this module owns the single
//! implementation to keep them in sync.

use std::collections::HashMap;

use bevy::math::Vec2;
use cordon_core::primitive::Id;
use cordon_core::world::area::{Area, AreaDef};
use rand::{Rng, RngExt};

/// Number of waypoints rolled per call. Three points is enough to
/// make a visible patrol ring without clumping inside the area.
pub const PATROL_RING_SIZE: usize = 3;

/// Roll [`PATROL_RING_SIZE`] scattered waypoints inside the given
/// area's disk. Returns an empty vec if the area is not in the
/// catalog — callers treat that as "authoring bug, stay idle"
/// rather than panic.
///
/// Points are pulled from a ring between 30% and 70% of the area's
/// radius so squads don't all converge on the centre, and don't
/// drift outside the visible disk. Angles are uniform over 2π.
pub fn roll_area_waypoints(
    area_id: &Id<Area>,
    areas: &HashMap<Id<Area>, AreaDef>,
    rng: &mut impl Rng,
) -> Vec<Vec2> {
    let Some(area) = areas.get(area_id) else {
        return Vec::new();
    };
    let cx = area.location.x;
    let cy = area.location.y;
    let r = area.radius.value() * 0.7;
    (0..PATROL_RING_SIZE)
        .map(|_| {
            let angle = rng.random_range(0.0_f32..std::f32::consts::TAU);
            let dist = rng.random_range(r * 0.3..r);
            Vec2::new(cx + angle.cos() * dist, cy + angle.sin() * dist)
        })
        .collect()
}
