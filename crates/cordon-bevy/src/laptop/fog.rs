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
use super::environment::fog::{FogMaterial, FogOverlay, MAX_DISCOVERED_AREAS, MAX_REVEAL_CIRCLES};
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

/// Cache of the per-frame reveal circles computed by [`apply_fog`].
/// Stored in a resource so [`sync_fog_material`] can write them
/// straight into the shader uniform without recomputing the union
/// over every player-squad member each frame.
#[derive(Resource, Default, Debug)]
pub struct FogReveals(pub Vec<(Vec2, f32)>);

/// Cache of the per-frame discovered area disks computed by
/// [`apply_fog`]. Each entry is `(centre, radius)`. Same shape
/// and lifecycle as [`FogReveals`].
#[derive(Resource, Default, Debug)]
pub struct DiscoveredDisks(pub Vec<(Vec2, f32)>);

/// Persistent breadcrumb trail of where the player's squads have
/// been. Sampled at low frequency by [`sample_memory_trail`] and
/// drawn as small "memory" disks in the fog shader so the player
/// can see the path their squads took even after they've moved on.
///
/// Capped to keep the discovered uniform array fitting comfortably
/// alongside the area disks; oldest entries are evicted first
/// once the cap is reached, so memory is "live last N samples"
/// rather than infinite.
#[derive(Resource, Default, Debug)]
pub struct MemoryTrail {
    /// `(centre, radius)` per breadcrumb. Stored in insertion
    /// order; the front is the oldest entry.
    pub points: std::collections::VecDeque<(Vec2, f32)>,
    /// Time-since-startup of the last sample, used to throttle.
    pub last_sample: f32,
}

/// Maximum number of breadcrumbs in the memory trail. Combined
/// with `MAX_AREAS_IN_DISCOVERED` (40 areas worst case) this stays
/// well under [`MAX_DISCOVERED_AREAS`].
const MAX_TRAIL_POINTS: usize = 200;

/// Radius of each breadcrumb in world units. A bit smaller than
/// a typical NPC vision circle so the trail reads as a corridor
/// rather than a series of fat dots.
const TRAIL_POINT_RADIUS: f32 = 90.0;

/// Wall-clock seconds between trail samples. Faster = more
/// granular trail, more crowded uniform array.
const TRAIL_SAMPLE_INTERVAL: f32 = 1.5;

pub struct FogPlugin;

impl Plugin for FogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerSquads>();
        app.init_resource::<RevealedAreas>();
        app.init_resource::<FogEnabled>();
        app.init_resource::<FogReveals>();
        app.init_resource::<DiscoveredDisks>();
        app.init_resource::<MemoryTrail>();
        app.add_systems(
            Update,
            (
                pick_player_squads,
                apply_fog,
                sample_memory_trail,
                sync_fog_material,
            )
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

/// World-space radius of the always-on "vision" around the bunker
/// at the origin. The player can see anything inside this circle
/// without scouting it — it's where they live, after all. Kept
/// small so it reads as "the immediate surroundings" rather than
/// a free chunk of map.
const BUNKER_REVEAL_RADIUS: f32 = 90.0;

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

    // Gather reveal circles from player-squad members. When fog is
    // disabled the loops below short-circuit to "visible" regardless.
    // We reuse the cache buffer rather than reallocating each frame.
    fog_reveals.0.clear();
    // Always-on vision around the bunker — the player can see what's
    // right around their home without scouting it.
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
    // Point/disk visibility tests used by NPCs and relics. When fog
    // is off everything is visible (cheat mode).
    let visible_point = |p: Vec2| -> bool {
        if !fog_on {
            return true;
        }
        reveals.iter().any(|(c, r)| c.distance_squared(p) <= r * r)
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
        // Once seen, an area marker stays on the map permanently.
        // The fog cheat (when off) reveals everything regardless.
        let show = !fog_on || is_discovered;
        set_vis(
            &mut vis,
            if show {
                Visibility::Visible
            } else {
                Visibility::Hidden
            },
        );
        // `visible_area_disks` is what the anomaly visibility pass
        // below uses to decide whether to render an anomaly's
        // shader effects. Anomalies should only run their flashy
        // shader when actively in sight — otherwise the player
        // could remember an anomaly forever via its visual.
        if in_sight {
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

/// Drop a breadcrumb at each player squad's centroid every
/// [`TRAIL_SAMPLE_INTERVAL`] seconds, evicting the oldest
/// breadcrumb when the buffer is full. The trail is then merged
/// into the discovered uniform array by [`sync_fog_material`].
///
/// We pick the centroid (not the leader, not a member) so the
/// breadcrumb sits in the middle of the squad's formation
/// regardless of where individual members are walking.
fn sample_memory_trail(
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
    let mut sums: std::collections::HashMap<Entity, (Vec2, u32)> = std::collections::HashMap::new();
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
        // than the breadcrumb radius, the new one would just sit
        // on top of it. Skipping saves slots in the capped buffer.
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

/// Push the cached reveal/discovered arrays into the fog overlay
/// material so the WGSL shader can render the right cut-throughs
/// each frame. Runs after [`apply_fog`] in the same chain so it
/// always sees the freshest data.
///
/// When fog is disabled (cheat mode), we publish a synthetic
/// reveal so the shader's "currently visible" mask covers the
/// whole map and effectively disables the overlay.
fn sync_fog_material(
    fog_reveals: Res<FogReveals>,
    discovered_disks: Res<DiscoveredDisks>,
    memory_trail: Res<MemoryTrail>,
    fog_enabled: Res<FogEnabled>,
    state: Res<State<PlayingState>>,
    active_tab: Res<LaptopTab>,
    mut overlay_q: Query<(&MeshMaterial2d<FogMaterial>, &mut Visibility), With<FogOverlay>>,
    mut fog_mats: ResMut<Assets<FogMaterial>>,
) {
    let map_visible = *state.get() == PlayingState::Laptop && *active_tab == LaptopTab::Map;
    let Ok((handle, mut overlay_vis)) = overlay_q.single_mut() else {
        return;
    };
    // Hide the overlay entirely while the player isn't looking at
    // the map. Change-guarded so the visibility write doesn't
    // dirty the render pipeline every frame.
    let target_vis = if map_visible {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    if *overlay_vis != target_vis {
        *overlay_vis = target_vis;
    }
    if !map_visible {
        return;
    }
    let Some(mat) = fog_mats.get_mut(&handle.0) else {
        return;
    };

    if !fog_enabled.enabled {
        // Cheat mode: one giant reveal disk that covers the whole
        // map. Cheaper than threading a separate "fog disabled"
        // branch through the shader.
        mat.counts = Vec4::new(1.0, 0.0, 0.0, 0.0);
        mat.reveals[0] = Vec4::new(0.0, 0.0, 1_000_000.0, 0.0);
        return;
    }

    // Pack reveal circles into the fixed-size uniform array. Any
    // slots beyond the active count are ignored by the shader (it
    // breaks the loop at `i >= counts.x`), but we still zero them
    // so stale data from a previous frame can't leak through if
    // the count somehow advances unchanged.
    let n_reveals = fog_reveals.0.len().min(MAX_REVEAL_CIRCLES);
    for (i, slot) in mat.reveals.iter_mut().enumerate() {
        *slot = if i < n_reveals {
            let (c, r) = fog_reveals.0[i];
            Vec4::new(c.x, c.y, r, 0.0)
        } else {
            Vec4::ZERO
        };
    }

    // Pack discovered area disks first, then breadcrumb trail
    // points, into the same uniform array. The shader doesn't
    // care which is which — both contribute to the memory wash.
    let n_areas = discovered_disks.0.len().min(MAX_DISCOVERED_AREAS);
    let trail_room = MAX_DISCOVERED_AREAS - n_areas;
    let n_trail = memory_trail.points.len().min(trail_room);
    let n_disc = n_areas + n_trail;

    for i in 0..MAX_DISCOVERED_AREAS {
        let value = if i < n_areas {
            let (c, r) = discovered_disks.0[i];
            Vec4::new(c.x, c.y, r, 0.0)
        } else if i < n_disc {
            let trail_idx = i - n_areas;
            // Walk from the *back* of the deque so the most-recent
            // breadcrumbs always make the cut when the trail is
            // longer than the remaining slots.
            let from_back = memory_trail.points.len() - 1 - trail_idx;
            let (c, r) = memory_trail.points[from_back];
            Vec4::new(c.x, c.y, r, 0.0)
        } else {
            Vec4::ZERO
        };
        mat.discovered[i] = value;
    }

    mat.counts = Vec4::new(n_reveals as f32, n_disc as f32, 0.0, 0.0);
}
