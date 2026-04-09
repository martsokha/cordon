//! Faction reactions to active events.
//!
//! Pure function over the active event log + the area state map and
//! player state. Called from the day-cycle systems in [`crate::day`].

use std::collections::HashMap;

use cordon_core::entity::player::PlayerState;
use cordon_core::primitive::{Id, RelationDelta};
use cordon_core::world::area::Area;
use cordon_core::world::narrative::{ActiveEvent, Event};

use crate::resources::AreaState;

/// Well-known event def IDs (must match config).
const EVENT_FACTION_WAR: &str = "faction_war";
const EVENT_COUP: &str = "coup";

/// Update faction dynamics based on active events.
///
/// - `faction_war`: bumps `danger_modifier` for areas controlled by
///   any of the warring factions.
/// - `coup`: applies a partial reset of the player's standing with
///   the involved faction.
pub fn tick_factions(
    events: &[ActiveEvent],
    areas: &mut HashMap<Id<Area>, AreaState>,
    player: &mut PlayerState,
) {
    let war_id = Id::<Event>::new(EVENT_FACTION_WAR);
    let coup_id = Id::<Event>::new(EVENT_COUP);

    for event in events {
        if event.def_id == war_id {
            for area in areas.values_mut() {
                if let Some(ref ctrl) = area.controlling_faction
                    && event.involved_factions.contains(ctrl)
                {
                    area.danger_modifier += 0.1;
                }
            }
        } else if event.def_id == coup_id
            && let Some(faction) = event.involved_factions.first()
            && let Some(standing) = player.standing_mut(faction)
        {
            let current = standing.value();
            let delta = -(current as f32 * 0.3) as i16;
            standing.apply(RelationDelta::new(delta));
        }
    }
}
