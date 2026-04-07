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
use cordon_core::entity::squad::{Formation, Goal};
use cordon_core::primitive::Id;
use cordon_core::world::area::Area;

use crate::components::{SquadFormation, SquadGoal, SquadId, SquadMarker, SquadWaypoints};

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
    /// Carry items to a destination point.
    Deliver { squad: Entity, to: Vec2 },
    /// Switch the marching formation.
    SetFormation { squad: Entity, formation: Formation },
}

/// Apply queued [`SquadCommand`]s to their target squads. Commands
/// targeting unowned squads are dropped silently — the UI is
/// responsible for not offering player commands against world
/// squads, and there is no other mutation API.
pub(super) fn apply_squad_commands(
    mut messages: MessageReader<SquadCommand>,
    owned: Query<(), (With<SquadMarker>, With<Owned>)>,
    squad_ids: Query<&SquadId>,
    mut squads: Query<(&mut SquadGoal, &mut SquadFormation, &mut SquadWaypoints)>,
) {
    for cmd in messages.read() {
        let target = cmd.squad();
        if owned.get(target).is_err() {
            continue;
        }
        let Ok((mut goal, mut formation, mut waypoints)) = squads.get_mut(target) else {
            continue;
        };

        match cmd {
            SquadCommand::Hold { .. } => {
                goal.0 = Goal::Idle;
                waypoints.points.clear();
                waypoints.next = 0;
            }
            SquadCommand::Patrol { area, .. } => {
                goal.0 = Goal::Patrol { area: area.clone() };
                // Concrete waypoints get rolled by the next squad-AI
                // tick — clear the old set so fresh ones land.
                waypoints.points.clear();
                waypoints.next = 0;
            }
            SquadCommand::Scavenge { area, .. } => {
                goal.0 = Goal::Scavenge { area: area.clone() };
                waypoints.points.clear();
                waypoints.next = 0;
            }
            SquadCommand::Protect { other, .. } => {
                // Goal::Protect stores a Uid<Squad> (save-game stable).
                // Resolve the runtime entity to its uid via the
                // SquadId component on the target.
                let Ok(other_id) = squad_ids.get(*other) else {
                    continue;
                };
                goal.0 = Goal::Protect { other: other_id.0 };
            }
            SquadCommand::Deliver { to, .. } => {
                goal.0 = Goal::Deliver { to: [to.x, to.y] };
                waypoints.points.clear();
                waypoints.next = 0;
            }
            SquadCommand::SetFormation {
                formation: new_formation,
                ..
            } => {
                formation.0 = *new_formation;
            }
        }
    }
}

impl SquadCommand {
    /// Target squad entity for any command variant.
    pub fn squad(&self) -> Entity {
        match self {
            SquadCommand::Hold { squad }
            | SquadCommand::Patrol { squad, .. }
            | SquadCommand::Scavenge { squad, .. }
            | SquadCommand::Protect { squad, .. }
            | SquadCommand::Deliver { squad, .. }
            | SquadCommand::SetFormation { squad, .. } => *squad,
        }
    }
}
