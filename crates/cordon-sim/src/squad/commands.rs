//! Player → sim command boundary.
//!
//! The player can only affect the simulated world by writing
//! [`SquadCommand`] messages from the UI. The [`apply_squad_commands`]
//! system below owns the only mutation path for player intent and
//! runs first in `SimSet::Commands`, so an order issued this frame
//! takes effect before any AI re-evaluation later in the same frame.
//!
//! Squad commands target only [`Owned`] squads. The apply system
//! silently drops commands aimed at world (non-owned) squads — quest
//! givers and hostile factions can never be remote-controlled because
//! there is no other API.

use bevy::prelude::*;
use bevy_prng::WyRand;
use bevy_rand::prelude::GlobalRng;
use cordon_core::entity::squad::{Formation, Goal, Squad};
use cordon_core::primitive::{Id, Uid};
use cordon_core::world::area::Area;
use cordon_data::gamedata::GameDataResource;
use rand::RngExt;

use crate::components::{SquadMarker, SquadWaypoints};

/// Marker for squads under direct player control. Only owned squads
/// react to [`SquadCommand`]s.
#[derive(Component, Debug, Clone, Copy)]
pub struct Owned;

/// What the player can ask one of their squads to do. Issued from the
/// laptop UI; applied by [`apply_squad_commands`].
#[derive(Message, Debug, Clone)]
pub enum SquadCommand {
    /// Drop everything and idle in place.
    Hold { squad: Entity },
    /// Patrol an area's waypoints.
    Patrol { squad: Entity, area: Id<Area> },
    /// Sweep an area for loot.
    Scavenge { squad: Entity, area: Id<Area> },
    /// Tail another squad and engage anything that attacks them.
    Protect { squad: Entity, other: Entity },
    /// Travel to a destination point.
    GoTo { squad: Entity, to: Vec2 },
    /// Switch the marching formation.
    SetFormation { squad: Entity, formation: Formation },
}

/// Apply queued [`SquadCommand`]s to their target squads. Commands
/// targeting unowned squads are dropped silently — the UI is
/// responsible for not offering player commands against world
/// squads, and there is no other mutation API.
///
/// Goals are changed via `commands.entity(squad).insert(new_goal)`
/// (not `*goal = new_goal`) so the `Insert<Goal>` observer fires and
/// the behavior-tree attach logic rebuilds the squad's tree.
/// Mutating in place would leave the old tree in charge.
pub(super) fn apply_squad_commands(
    mut messages: MessageReader<SquadCommand>,
    data: Res<GameDataResource>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
    mut cmds: Commands,
    owned: Query<(), (With<SquadMarker>, With<Owned>)>,
    squad_ids: Query<&Uid<Squad>>,
    mut squads: Query<(&mut Formation, &mut SquadWaypoints)>,
) {
    for cmd in messages.read() {
        let target = cmd.squad();
        if owned.get(target).is_err() {
            continue;
        }
        let Ok((mut formation, mut waypoints)) = squads.get_mut(target) else {
            continue;
        };

        match cmd {
            SquadCommand::Hold { .. } => {
                waypoints.points.clear();
                waypoints.next = 0;
                cmds.entity(target).insert(Goal::Idle);
            }
            SquadCommand::Patrol { area, .. } => {
                // Roll concrete waypoints inside the area so Patrol
                // actually moves. Pre-port this was left to a
                // squad-AI tick that never existed, so player-issued
                // Patrol stalled with an empty list.
                waypoints.points = roll_area_waypoints(area, &data, rng.as_mut());
                waypoints.next = 0;
                cmds.entity(target).insert(Goal::Patrol { area: area.clone() });
            }
            SquadCommand::Scavenge { area, .. } => {
                waypoints.points = roll_area_waypoints(area, &data, rng.as_mut());
                waypoints.next = 0;
                cmds.entity(target).insert(Goal::Scavenge { area: area.clone() });
            }
            SquadCommand::Protect { other, .. } => {
                // Goal::Protect stores a Uid<Squad> (save-game stable).
                // Resolve the runtime entity to its uid via the
                // Uid<Squad> component on the target.
                let Ok(other_uid) = squad_ids.get(*other) else {
                    continue;
                };
                cmds.entity(target).insert(Goal::Protect { other: *other_uid });
            }
            SquadCommand::GoTo { to, .. } => {
                waypoints.points.clear();
                waypoints.next = 0;
                cmds.entity(target).insert(Goal::GoTo {
                    target: [to.x, to.y],
                    intent: cordon_core::entity::squad::TravelIntent::Generic,
                });
            }
            SquadCommand::SetFormation {
                formation: new_formation,
                ..
            } => {
                *formation = *new_formation;
            }
        }
    }
}

/// Roll 3 scattered waypoints inside an area. Mirrors
/// `spawn::generator::waypoints_for_goal` so player-issued Patrol
/// lands the same shape of ring as spawn-generated patrol squads.
fn roll_area_waypoints(
    area_id: &Id<Area>,
    data: &GameDataResource,
    rng: &mut WyRand,
) -> Vec<Vec2> {
    let Some(area) = data.0.areas.get(area_id) else {
        return Vec::new();
    };
    let cx = area.location.x;
    let cy = area.location.y;
    let r = area.radius.value() * 0.7;
    (0..3)
        .map(|_| {
            let angle = rng.random_range(0.0_f32..std::f32::consts::TAU);
            let dist = rng.random_range(r * 0.3..r);
            Vec2::new(cx + angle.cos() * dist, cy + angle.sin() * dist)
        })
        .collect()
}

impl SquadCommand {
    /// Target squad entity for any command variant.
    pub fn squad(&self) -> Entity {
        match self {
            SquadCommand::Hold { squad }
            | SquadCommand::Patrol { squad, .. }
            | SquadCommand::Scavenge { squad, .. }
            | SquadCommand::Protect { squad, .. }
            | SquadCommand::GoTo { squad, .. }
            | SquadCommand::SetFormation { squad, .. } => *squad,
        }
    }
}
