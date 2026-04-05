use cordon_core::world::event::EventKind;

use crate::state::world::World;

/// Update faction dynamics based on active events.
pub fn tick_factions(world: &mut World) {
    // Collect events that need processing to avoid borrow issues.
    let events: Vec<_> = world.active_events.iter().map(|e| e.kind.clone()).collect();

    for kind in events {
        match &kind {
            EventKind::FactionWar(a, b) => {
                // Warring factions increase danger in sectors they control
                for sector in world.sectors.values_mut() {
                    if let Some(ref ctrl) = sector.controlling_faction {
                        if ctrl == a || ctrl == b {
                            sector.danger_modifier += 0.1;
                        }
                    }
                }
            }
            EventKind::Coup(faction) => {
                // Partial standing reset with the player
                if let Some(standing) = world.player.standing_mut(faction) {
                    let current = standing.value();
                    let delta = -(current as f32 * 0.3) as i8;
                    standing.apply(delta);
                }
            }
            _ => {}
        }
    }
}
