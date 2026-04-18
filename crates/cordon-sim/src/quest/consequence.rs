//! [`Consequence`] application.
//!
//! Single free function that pattern-matches a [`Consequence`]
//! and writes the corresponding mutation. Quest outcomes, event
//! triggers, and choice effects all route through [`apply`].

use bevy::prelude::*;
use bevy_prng::WyRand;
use cordon_core::entity::faction::Faction;
use cordon_core::item::ItemInstance;
use cordon_core::primitive::{GameTime, Id};
use cordon_core::world::narrative::{ActiveEvent, Consequence};
use cordon_data::catalog::GameData;

use super::messages::{
    EndGameRequest, GiveNpcXpRequest, SpawnNpcRequest, StandingChanged, StartQuestRequest,
};
use super::registry::TemplateRegistry;
use crate::day::world_events::{EventOverrides, spawn_event_instance};
use crate::resources::{PlayerIdentity, PlayerIntel, PlayerStandings, PlayerStash, PlayerUpgrades};

/// Apply a single consequence. Warnings rather than panics so
/// content typos don't crash the sim.
pub fn apply(
    consequence: &Consequence,
    identity: &mut PlayerIdentity,
    standings: &mut PlayerStandings,
    upgrades: &mut PlayerUpgrades,
    stash: &mut PlayerStash,
    intel: &mut PlayerIntel,
    events: &mut Vec<ActiveEvent>,
    data: &GameData,
    registry: &TemplateRegistry,
    now: GameTime,
    rng: &mut WyRand,
    faction_pool: &[Id<Faction>],
    start_quest: &mut MessageWriter<StartQuestRequest>,
    spawn_npc: &mut MessageWriter<SpawnNpcRequest>,
    give_npc_xp: &mut MessageWriter<GiveNpcXpRequest>,
    standing_changed: &mut MessageWriter<StandingChanged>,
    end_game: &mut MessageWriter<EndGameRequest>,
) {
    match consequence {
        Consequence::StandingChange { faction, delta } => {
            if let Some(standing) = standings.standing_mut(faction) {
                standing.apply(*delta);
                standing_changed.write(StandingChanged {
                    faction: faction.clone(),
                    delta: *delta,
                });
            } else {
                warn!("StandingChange: unknown faction `{}`", faction.as_str());
            }
        }

        Consequence::GiveCredits(amount) => {
            identity.credits += *amount;
        }

        Consequence::TakeCredits(amount) => {
            identity.credits -= *amount;
        }

        Consequence::GiveItem(q) => {
            let Some(def) = data.item(&q.item) else {
                warn!("GiveItem: unknown item `{}`", q.item.as_str());
                return;
            };
            let count = q.resolved_count();
            for _ in 0..count {
                stash.add_item(ItemInstance::new(def), q.scope);
            }
        }

        Consequence::TakeItem(q) => {
            let count = q.resolved_count();
            let mut removed = 0u32;
            for _ in 0..count {
                if stash.remove_first(&q.item, q.scope).is_none() {
                    break;
                }
                removed += 1;
            }
            if removed < count {
                warn!(
                    "TakeItem: wanted {count}× `{}` in scope {:?}, only removed {removed}",
                    q.item.as_str(),
                    q.scope
                );
            }
        }

        Consequence::TriggerEvent {
            event,
            target_area,
            involved_factions,
            duration_days,
        } => {
            let Some(def) = data.events.get(event) else {
                warn!("TriggerEvent: unknown event `{}`", event.as_str());
                return;
            };
            let overrides = EventOverrides {
                target_area: target_area.clone(),
                involved_factions: involved_factions.clone(),
                duration_days: *duration_days,
            };
            let instance = spawn_event_instance(def, faction_pool, now.day, &overrides, rng);
            events.push(instance);
        }

        Consequence::StartQuest(quest_id) => {
            start_quest.write(StartQuestRequest {
                quest: quest_id.clone(),
            });
        }

        Consequence::UnlockUpgrade(upgrade) => {
            if !upgrades.upgrades.contains(upgrade) {
                upgrades.upgrades.push(upgrade.clone());
            }
        }

        Consequence::GiveIntel(intel_id) => {
            intel.grant(intel_id.clone(), now.day);
        }

        Consequence::SpawnNpc { template, at } => {
            let Some(def) = data.npc_template(template) else {
                warn!("SpawnNpc: unknown template `{}`", template.as_str());
                return;
            };
            if def.unique && registry.is_alive(template) {
                info!(
                    "SpawnNpc: unique template `{}` already alive, skipping",
                    template.as_str()
                );
                return;
            }
            if !def.respawnable && registry.is_permanently_dead(template) {
                info!(
                    "SpawnNpc: non-respawnable template `{}` is permanently dead, skipping",
                    template.as_str()
                );
                return;
            }
            spawn_npc.write(SpawnNpcRequest {
                template: template.clone(),
                at: at.clone(),
                yarn_node: None,
            });
        }

        Consequence::GivePlayerXp(xp) => {
            identity.add_xp(xp.value());
        }

        Consequence::GiveNpcXp { template, amount } => {
            if !registry.is_alive(template) {
                warn!(
                    "GiveNpcXp: template `{}` is not alive, cannot grant {} xp",
                    template.as_str(),
                    amount.value(),
                );
                return;
            }
            give_npc_xp.write(GiveNpcXpRequest {
                template: template.clone(),
                amount: *amount,
            });
        }

        Consequence::DangerModifier {
            area,
            delta,
            duration,
        } => {
            let target = area
                .as_ref()
                .map(|a| a.as_str().to_string())
                .unwrap_or_else(|| "zone-wide".to_string());
            let lifetime = duration
                .map(|d| d.to_string())
                .unwrap_or_else(|| "permanent".to_string());
            warn!("STUB `danger_modifier`: area `{target}`, delta {delta}, lifetime {lifetime}.");
        }

        Consequence::PriceModifier {
            category,
            multiplier,
            duration,
        } => {
            let lifetime = duration
                .map(|d| d.to_string())
                .unwrap_or_else(|| "permanent".to_string());
            warn!("STUB `price_modifier`: {category:?} ×{multiplier}, lifetime {lifetime}.");
        }

        Consequence::EndGame { cause } => {
            info!("EndGame: cause {cause:?}");
            end_game.write(EndGameRequest { cause: *cause });
        }
    }
}
