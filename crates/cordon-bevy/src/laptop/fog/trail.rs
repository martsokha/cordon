//! Memory trail sampler.
//!
//! Drops a breadcrumb at each player squad's centroid every
//! [`TRAIL_SAMPLE_INTERVAL`] seconds, evicting the oldest when
//! the ring fills. The trail is packed into the fog shader's
//! `discovered` uniform array by [`super::sync::sync_fog_material`]
//! so the path the squad walked stays visible as a grey memory
//! wash even after they've moved on.

use std::collections::HashMap;

use bevy::prelude::*;
use cordon_sim::components::{NpcMarker, SquadMembership};

use super::{MemoryTrail, PlayerSquads};

/// Maximum number of breadcrumbs in the memory trail. Combined
/// with discovered areas this stays well under the fog shader's
/// `MAX_DISCOVERED_AREAS` slot count.
const MAX_TRAIL_POINTS: usize = 200;

/// Radius of each breadcrumb in world units. A bit smaller than a
/// typical NPC vision circle so the trail reads as a corridor
/// rather than a series of fat dots.
const TRAIL_POINT_RADIUS: f32 = 90.0;

/// Seconds between trail samples at 1× time scale. At higher time
/// scales the sampler fires more often in wall-clock time but the
/// same in sim-time, which is what we want — more breadcrumbs per
/// real second since squads are covering more ground.
const TRAIL_SAMPLE_INTERVAL: f32 = 1.5;

/// Drop a breadcrumb at each player squad's centroid every
/// [`TRAIL_SAMPLE_INTERVAL`] seconds, evicting the oldest
/// breadcrumb when the buffer is full.
///
/// We pick the centroid (not the leader, not a member) so the
/// breadcrumb sits in the middle of the squad's formation
/// regardless of where individual members are walking.
pub(super) fn sample_memory_trail(
    time: Res<Time>,
    player_squads: Res<PlayerSquads>,
    members: Query<
        (&Transform, &SquadMembership),
        (With<NpcMarker>, Without<cordon_sim::behavior::Dead>),
    >,
    mut trail: ResMut<MemoryTrail>,
) {
    let now = time.elapsed_secs();
    if now - trail.last_sample < TRAIL_SAMPLE_INTERVAL {
        return;
    }
    trail.last_sample = now;

    // Centroid per player squad: sum of member positions / count.
    // A `HashMap` keyed on the squad entity handles this in one
    // pass over the member list.
    let mut sums: HashMap<Entity, (Vec2, u32)> = HashMap::new();
    for (transform, membership) in &members {
        if !player_squads.0.contains(&membership.squad) {
            continue;
        }
        let pos = transform.translation.truncate();
        let entry = sums.entry(membership.squad).or_insert((Vec2::ZERO, 0));
        entry.0 += pos;
        entry.1 += 1;
    }

    for (_squad, (sum, count)) in sums {
        if count == 0 {
            continue;
        }
        let centroid = sum / count as f32;
        // Don't append duplicate breadcrumbs when the squad is
        // standing still — if the most-recent breadcrumb is closer
        // than half the breadcrumb radius, the new one would just
        // stack on top of it. Skipping saves slots in the ring.
        if let Some(&(prev, _)) = trail.points.back()
            && prev.distance_squared(centroid) < (TRAIL_POINT_RADIUS * 0.5).powi(2)
        {
            continue;
        }
        if trail.points.len() >= MAX_TRAIL_POINTS {
            trail.points.pop_front();
        }
        trail.points.push_back((centroid, TRAIL_POINT_RADIUS));
    }
}
