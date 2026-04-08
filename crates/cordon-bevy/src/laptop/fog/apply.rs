//! Per-frame visibility application.
//!
//! Builds the set of reveal circles from live player-squad members
//! + the bunker, then walks every area / NPC / relic / anomaly
//! visual and toggles `Visibility` based on whether it intersects
//! any reveal. Visibility writes are change-guarded so a stable
//! scene doesn't dirty Bevy's change detection cascade.

use bevy::prelude::*;
use cordon_sim::components::{NpcMarker, RelicMarker, SquadMembership};

use super::{DiscoveredDisks, FogEnabled, FogReveals, PlayerSquads, RevealedAreas};
use crate::PlayingState;
use crate::laptop::environment::anomaly::AnomalyVisual;
use crate::laptop::ui::LaptopTab;
use crate::laptop::{AreaCircle, Bunker};

/// World-space radius of the always-on vision around the bunker
/// at the origin. The player can see anything inside this circle
/// without scouting it — it's where they live, after all. Kept
/// small so it reads as "the immediate surroundings" rather than
/// a free chunk of map.
const BUNKER_REVEAL_RADIUS: f32 = 90.0;

/// Writing a `Visibility` component (even to the same value)
/// marks it as changed, which propagates to children and dirties
/// downstream render commands. Guarded write.
fn set_vis(vis: &mut Mut<Visibility>, target: Visibility) {
    if **vis != target {
        **vis = target;
    }
}

/// Apply the fog to every map-world entity each frame.
///
/// Reveal circles are the union of (member transform, member
/// vision) for every non-dead NPC belonging to a player squad,
/// plus a small always-on circle around the bunker. Anything
/// inside any circle is visible; everything else is hidden —
/// except the bunker dot, which is always visible, and areas,
/// which latch into [`RevealedAreas`] the first time they're
/// seen so their marker persists forever.
#[allow(clippy::type_complexity)]
pub(super) fn apply_fog(
    player_squads: Res<PlayerSquads>,
    mut revealed_areas: ResMut<RevealedAreas>,
    mut fog_reveals: ResMut<FogReveals>,
    mut discovered_disks: ResMut<DiscoveredDisks>,
    fog_enabled: Res<FogEnabled>,
    state: Res<State<PlayingState>>,
    active_tab: Res<LaptopTab>,
    members: Query<
        (&Transform, &SquadMembership, &cordon_sim::behavior::Vision),
        // Dead NPCs (corpses) shouldn't see anything — their
        // vision circle would otherwise reveal a chunk of fog
        // around their corpse forever.
        (With<NpcMarker>, Without<cordon_sim::behavior::Dead>),
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
    let map_visible = *state.get() == PlayingState::Laptop && *active_tab == LaptopTab::Map;
    if !map_visible {
        return;
    }

    // Gather reveal circles from player-squad members. We reuse
    // the cache buffer rather than reallocating each frame — it's
    // read back by `sync_fog_material` after this system finishes.
    fog_reveals.0.clear();
    fog_reveals.0.push((Vec2::ZERO, BUNKER_REVEAL_RADIUS));
    for (transform, membership, vision) in &members {
        if player_squads.0.contains(&membership.squad) {
            fog_reveals
                .0
                .push((transform.translation.truncate(), vision.radius));
        }
    }
    let reveals = &fog_reveals.0;
    let fog_on = fog_enabled.enabled;

    // Point/disk visibility tests. When fog is off (cheat mode)
    // `visible_point` returns true unconditionally. `latches_area`
    // always respects `fog_on` so toggling the cheat doesn't
    // retroactively "discover" areas the player hasn't scouted.
    let visible_point = |p: Vec2| -> bool {
        if !fog_on {
            return true;
        }
        reveals.iter().any(|(c, r)| c.distance_squared(p) <= r * r)
    };
    let latches_area = |p: Vec2, disk_r: f32| -> bool {
        if !fog_on {
            return false;
        }
        reveals.iter().any(|(c, r)| c.distance(p) <= r + disk_r)
    };

    // Areas have two states for the *mesh*:
    //
    //   - Never seen: hidden under the fog overlay entirely.
    //   - Ever seen: mesh + border stay visible *forever*. Whether
    //     the player can actually see it through the fog is
    //     decided by the fog shader sitting on top — areas that
    //     are currently in sight get a clear cut-through; areas
    //     that have been seen but aren't currently lit appear as
    //     darkened-but-still-rendered shapes through the fog.
    //
    // We deliberately do NOT push area disks into the discovered
    // set for the fog shader — the memory wash should follow the
    // breadcrumb trail of where the squad has actually walked,
    // not the entire abstract area outline.
    discovered_disks.0.clear();
    let mut visible_area_disks: Vec<(Vec2, f32)> = Vec::new();
    for (entity, transform, circle, mut vis) in &mut area_q {
        let p = transform.translation.truncate();
        let in_sight = latches_area(p, circle.radius);
        if in_sight {
            revealed_areas.0.insert(entity);
        }
        let is_discovered = revealed_areas.0.contains(&entity);
        let show = !fog_on || is_discovered;
        set_vis(
            &mut vis,
            if show {
                Visibility::Visible
            } else {
                Visibility::Hidden
            },
        );
        // `visible_area_disks` is what the anomaly visibility
        // pass below uses. Anomalies should only run their flashy
        // shader when actively in sight — otherwise the player
        // could remember an anomaly forever via its visual.
        if in_sight {
            visible_area_disks.push((p, circle.radius));
        }
    }

    // Anomaly shader visuals: only show if the anomaly's centre
    // sits inside a currently-visible area disk. Anomalies always
    // live inside their parent area by construction.
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
