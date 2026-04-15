//! Player-squad roster bookkeeping: picking starter squads and
//! keeping the [`Owned`] component in sync with the
//! [`PlayerSquadRoster`] resource.
//!
//! The roster is the authoritative, save-stable source of truth
//! (keyed by `Uid<Squad>`); the [`Owned`] marker is a derived
//! per-entity cache that hot-path queries can use cheaply
//! (`Query<.., With<Owned>>`).

use bevy::prelude::*;
use cordon_core::primitive::Uid;

use super::commands::Owned;
use super::identity::SquadMarker;
use crate::entity::npc::FactionId;
use crate::resources::{PlayerSquadRoster, SquadIdIndex};

/// Number of squads the player starts owning. Pulled from the
/// drifter faction so there's always something to pick from —
/// drifters are the neutral, always-present faction.
const STARTER_SQUAD_COUNT: usize = 3;

/// Pick a few drifter squads to be the player's once the sim has
/// finished spawning. Idempotent — bails if the roster is already
/// non-empty. Temporary scaffolding for testing; real ownership
/// will come from the hire flow.
pub fn pick_player_squads(
    mut roster: ResMut<PlayerSquadRoster>,
    squads: Query<(&Uid<cordon_core::entity::squad::Squad>, &FactionId), With<SquadMarker>>,
) {
    if !roster.is_empty() {
        return;
    }

    // Collect drifter squads first; if there are none yet, bail
    // and try again next frame. The sim sometimes takes a couple
    // of frames to finish spawning.
    let mut candidates: Vec<Uid<cordon_core::entity::squad::Squad>> = squads
        .iter()
        .filter(|(_, f)| f.0.as_str() == "faction_drifters")
        .map(|(uid, _)| *uid)
        .collect();
    if candidates.is_empty() {
        return;
    }

    // Deterministic pick: sort by uid value then stride. Real
    // randomness isn't needed here and bringing in a dep just
    // for this would be overkill — a different set every run
    // would also ruin reproducibility.
    candidates.sort_by_key(|u| u.value());
    let step = (candidates.len() / STARTER_SQUAD_COUNT.max(1)).max(1);
    for (i, uid) in candidates.into_iter().step_by(step).enumerate() {
        if i >= STARTER_SQUAD_COUNT {
            break;
        }
        roster.hire(uid);
    }

    info!(
        "roster: picked {} starter squads from drifters",
        roster.len()
    );
}

/// Reconcile the [`Owned`] marker on squad entities with the
/// authoritative [`PlayerSquadRoster`].
///
/// Runs every frame; the work is proportional to the roster size
/// (typically a handful of squads), not to the world population.
/// Squad entities in the roster gain `Owned` if they don't have
/// it; entities marked `Owned` whose Uid is no longer in the
/// roster lose the marker.
pub fn sync_owned_marker(
    mut commands: Commands,
    roster: Res<PlayerSquadRoster>,
    index: Res<SquadIdIndex>,
    owned_q: Query<(Entity, &Uid<cordon_core::entity::squad::Squad>), With<Owned>>,
) {
    // Add Owned to entities whose Uid is in the roster.
    for (uid, _) in roster.iter() {
        let Some(entity) = index.0.get(uid) else {
            continue;
        };
        commands.entity(*entity).insert(Owned);
    }
    // Strip Owned from entities whose Uid was dismissed.
    for (entity, uid) in &owned_q {
        if !roster.is_hired(uid) {
            commands.entity(entity).remove::<Owned>();
        }
    }
}
