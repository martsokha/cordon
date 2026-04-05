use cordon_core::event::EventKind;
use cordon_core::faction::FactionId;

use crate::world::World;

/// Update faction dynamics based on active events.
pub fn tick_factions(world: &mut World) {
    for event in &world.active_events {
        match &event.kind {
            EventKind::FactionWar(a, b) => {
                // Warring factions shift sector control
                if let Some(sector) = world
                    .sectors
                    .values_mut()
                    .find(|s| s.controlling_faction == Some(*a) || s.controlling_faction == Some(*b))
                {
                    sector.danger_modifier += 0.1;
                }
            }
            EventKind::Coup(faction) => {
                // Partial standing reset with the player
                let standing = world.player.standing_mut(*faction);
                let current = standing.value();
                // Regress toward neutral by 30%
                let delta = -(current as f32 * 0.3) as i8;
                standing.apply(delta);
            }
            _ => {}
        }
    }
}

/// Compute standing ripple effects when player trades with a faction.
pub fn standing_ripple(faction: FactionId) -> Vec<(FactionId, i8)> {
    match faction {
        FactionId::Order => vec![
            (FactionId::Collective, -3),
        ],
        FactionId::Collective => vec![
            (FactionId::Order, -3),
            (FactionId::Garrison, -1),
        ],
        FactionId::Syndicate => vec![
            (FactionId::Order, -2),
            (FactionId::Garrison, -2),
        ],
        FactionId::Garrison => vec![
            (FactionId::Syndicate, -1),
        ],
        FactionId::Institute => vec![
            (FactionId::Devoted, -2),
        ],
        FactionId::Devoted => vec![
            (FactionId::Institute, -3),
        ],
        FactionId::Drifters | FactionId::Mercenaries => vec![],
    }
}
