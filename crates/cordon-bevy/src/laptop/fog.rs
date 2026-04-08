//! Fog of war for the laptop map.
//!
//! The player "owns" a small set of squads (randomly picked at
//! sim-startup for now — eventually these will be the squads the
//! player recruits). Everything on the map is hidden except within
//! the vision radius of a player-owned squad member.
//!
//! - **NPC dots and relics** reveal in real time. Step out of a
//!   player squad's vision → instantly dark again.
//! - **Areas** are persistent: the first time a player squad lays
//!   eyes on one, it stays on the minimap forever. Intel doesn't
//!   un-learn itself.
//! - The **bunker** dot is always visible.
//! - Player squad members themselves are always visible.

use std::collections::HashSet;

use bevy::prelude::*;
use cordon_sim::components::{NpcMarker, RelicMarker, SquadFaction, SquadMarker, SquadMembership};

use super::environment::anomaly::AnomalyVisual;
use super::ui::LaptopTab;
use super::{AreaCircle, Bunker};
use crate::PlayingState;

/// Squads the player commands. Membership is set once by
/// [`pick_player_squads`] and never changes afterward (for now).
/// Every fog-related system filters through this set.
#[derive(Resource, Default, Debug)]
pub struct PlayerSquads(pub HashSet<Entity>);

/// Areas that have ever been in sight of a player squad. Once an
/// area enters this set it stays forever — scouting intel doesn't
/// decay. NPCs and relics inside that area still hide/show in real
/// time; only the area disk itself is persistent.
#[derive(Resource, Default, Debug)]
pub struct RevealedAreas(pub HashSet<Entity>);

/// Master fog toggle. When `enabled = false`, everything on the map
/// is visible regardless of player squad line-of-sight — useful
/// for debugging and for a player-facing "full map" reveal toggle.
/// Bound to F3 by [`toggle_fog`].
#[derive(Resource, Debug)]
pub struct FogEnabled {
    pub enabled: bool,
}

impl Default for FogEnabled {
    fn default() -> Self {
        Self { enabled: true }
    }
}

pub struct FogPlugin;

impl Plugin for FogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerSquads>();
        app.init_resource::<RevealedAreas>();
        app.init_resource::<FogEnabled>();
        app.add_systems(
            Update,
            (pick_player_squads, apply_fog)
                .chain()
                .after(cordon_sim::plugin::SimSet::Spawn)
                .run_if(in_state(crate::AppState::Playing)),
        );
    }
}

/// Number of squads the player starts owning. Pulled from the
/// drifter faction so there's always something to pick from — the
/// drifters are the neutral, always-present faction.
const PLAYER_SQUAD_COUNT: usize = 3;

/// Pick a few drifter squads to be the player's once the sim has
/// finished spawning. Idempotent — bails if the set is already
/// non-empty.
fn pick_player_squads(
    mut player_squads: ResMut<PlayerSquads>,
    squads: Query<(Entity, &SquadFaction), With<SquadMarker>>,
) {
    if !player_squads.0.is_empty() {
        return;
    }

    // Collect drifter squads first; if there are none yet, bail and
    // try again next frame. The sim sometimes takes a couple of
    // frames to finish spawning.
    let mut candidates: Vec<Entity> = squads
        .iter()
        .filter(|(_, f)| f.0.as_str() == "drifters")
        .map(|(e, _)| e)
        .collect();
    if candidates.is_empty() {
        return;
    }

    // Deterministic pick: sort by entity bits then stride. Real
    // randomness isn't needed here and bringing in a dep just for
    // this would be overkill — picking a different set every run
    // would also ruin reproducibility.
    candidates.sort_by_key(|e| e.to_bits());
    let step = (candidates.len() / PLAYER_SQUAD_COUNT.max(1)).max(1);
    for (i, entity) in candidates.into_iter().step_by(step).enumerate() {
        if i >= PLAYER_SQUAD_COUNT {
            break;
        }
        player_squads.0.insert(entity);
    }

    info!(
        "fog: picked {} player squads from drifters",
        player_squads.0.len()
    );
}

/// Apply the fog to every [`MapWorldEntity`] each frame.
///
/// Reveal circles are the union of (member transform, member
/// vision) for every NPC belonging to a player squad. Anything
/// inside any circle is visible; everything else is hidden — except
/// the bunker dot, which is always visible, and areas, which latch
/// into [`RevealedAreas`] the first time they're seen.
#[allow(clippy::type_complexity)]
fn apply_fog(
    player_squads: Res<PlayerSquads>,
    mut revealed_areas: ResMut<RevealedAreas>,
    fog_enabled: Res<FogEnabled>,
    state: Res<State<PlayingState>>,
    active_tab: Res<LaptopTab>,
    members: Query<
        (
            &Transform,
            &SquadMembership,
            &cordon_sim::behavior::Vision,
        ),
        With<NpcMarker>,
    >,
    mut area_q: Query<
        (Entity, &Transform, &AreaCircle, &mut Visibility),
        (Without<NpcMarker>, Without<RelicMarker>, Without<Bunker>),
    >,
    mut npc_q: Query<
        (&Transform, &SquadMembership, &mut Visibility),
        (With<NpcMarker>, Without<AreaCircle>, Without<Bunker>),
    >,
    mut relic_q: Query<
        (&Transform, &mut Visibility),
        (
            With<RelicMarker>,
            Without<NpcMarker>,
            Without<AreaCircle>,
            Without<Bunker>,
        ),
    >,
    mut anomaly_q: Query<
        (&Transform, &mut Visibility),
        (
            With<AnomalyVisual>,
            Without<RelicMarker>,
            Without<NpcMarker>,
            Without<AreaCircle>,
            Without<Bunker>,
        ),
    >,
) {
    // Only run the fog-driven visibility override while the player
    // is actually looking at the map. On other tabs/states the
    // normal MapOnlyUi / tab-switch visibility plumbing owns these
    // entities and we'd fight with it.
    let map_visible =
        *state.get() == PlayingState::Laptop && *active_tab == LaptopTab::Map;
    if !map_visible {
        return;
    }

    // Gather reveal circles from player-squad members. When fog is
    // disabled the loops below short-circuit to "visible" regardless.
    let mut reveals: Vec<(Vec2, f32)> = Vec::new();
    for (transform, membership, vision) in &members {
        if player_squads.0.contains(&membership.squad) {
            reveals.push((transform.translation.truncate(), vision.radius));
        }
    }

    let fog_on = fog_enabled.enabled;
    // Point/disk visibility tests used by NPCs and relics. When fog
    // is off everything is visible (cheat mode).
    let visible_point = |p: Vec2| -> bool {
        if !fog_on {
            return true;
        }
        reveals
            .iter()
            .any(|(c, r)| c.distance_squared(p) <= r * r)
    };
    // Used only for the "first time a player squad sees this area"
    // latch — *always* respects fog.on, so toggling the fog cheat
    // doesn't retroactively "discover" areas the player hasn't
    // actually scouted.
    let latches_area = |p: Vec2, disk_r: f32| -> bool {
        if !fog_on {
            return false;
        }
        reveals.iter().any(|(c, r)| c.distance(p) <= r + disk_r)
    };

    // Areas: latch into RevealedAreas the first time an actual
    // player squad sees them. When fog is off, *all* areas become
    // visible without touching the latch — so toggling back on
    // leaves only the areas the player has genuinely scouted.
    //
    // We also collect `(centre, radius)` for every currently
    // visible area into `visible_area_disks`, so the anomaly pass
    // below can decide which anomaly visuals to reveal without
    // re-querying the area data.
    // Writing a `Visibility` component (even to the same value)
    // marks it as changed, which propagates to children and dirties
    // downstream render commands. So: only write when the new value
    // actually differs from the current one. This cuts several
    // hundred per-frame writes to zero once the scene stabilises.
    fn set_vis(vis: &mut Mut<Visibility>, target: Visibility) {
        if **vis != target {
            **vis = target;
        }
    }

    let mut visible_area_disks: Vec<(Vec2, f32)> = Vec::new();
    for (entity, transform, circle, mut vis) in &mut area_q {
        let p = transform.translation.truncate();
        if !revealed_areas.0.contains(&entity) && latches_area(p, circle.radius) {
            revealed_areas.0.insert(entity);
        }
        let show = !fog_on || revealed_areas.0.contains(&entity);
        set_vis(
            &mut vis,
            if show {
                Visibility::Visible
            } else {
                Visibility::Hidden
            },
        );
        if show {
            visible_area_disks.push((p, circle.radius));
        }
    }

    // Anomaly shader visuals: only show if the anomaly's center
    // sits inside a currently-visible area disk. Anomalies always
    // live inside their parent area by construction, so "area
    // discovered" == "anomaly visible".
    for (transform, mut vis) in &mut anomaly_q {
        let p = transform.translation.truncate();
        let inside = visible_area_disks
            .iter()
            .any(|(c, r)| c.distance_squared(p) <= r * r);
        set_vis(
            &mut vis,
            if inside {
                Visibility::Visible
            } else {
                Visibility::Hidden
            },
        );
    }

    // NPCs: real-time. Player-owned squad members are always
    // visible (they're what's doing the revealing).
    for (transform, membership, mut vis) in &mut npc_q {
        let is_mine = player_squads.0.contains(&membership.squad);
        let p = transform.translation.truncate();
        set_vis(
            &mut vis,
            if is_mine || visible_point(p) {
                Visibility::Visible
            } else {
                Visibility::Hidden
            },
        );
    }

    // Relics: real-time.
    for (transform, mut vis) in &mut relic_q {
        let p = transform.translation.truncate();
        set_vis(
            &mut vis,
            if visible_point(p) {
                Visibility::Visible
            } else {
                Visibility::Hidden
            },
        );
    }
}
