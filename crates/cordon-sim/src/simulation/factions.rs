use cordon_core::primitive::id::Id;

use crate::state::world::World;

/// Well-known event def IDs (must match config).
const EVENT_FACTION_WAR: &str = "faction_war";
const EVENT_COUP: &str = "coup";

/// Update faction dynamics based on active events.
pub fn tick_factions(world: &mut World) {
    let war_id = Id::new(EVENT_FACTION_WAR);
    let coup_id = Id::new(EVENT_COUP);

    // Collect event data to avoid borrow issues.
    let event_data: Vec<_> = world
        .active_events
        .iter()
        .map(|e| (e.def_id.clone(), e.involved_factions.clone()))
        .collect();

    for (def_id, factions) in event_data {
        if def_id == war_id {
            // Warring factions increase danger in sectors they control
            for sector in world.sectors.values_mut() {
                if let Some(ref ctrl) = sector.controlling_faction {
                    if factions.contains(ctrl) {
                        sector.danger_modifier += 0.1;
                    }
                }
            }
        } else if def_id == coup_id {
            // Partial standing reset with the player
            if let Some(faction) = factions.first() {
                if let Some(standing) = world.player.standing_mut(faction) {
                    let current = standing.value();
                    let delta = -(current as f32 * 0.3) as i8;
                    standing.apply(delta);
                }
            }
        }
    }
}
