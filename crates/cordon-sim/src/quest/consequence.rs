//! [`Consequence`] application.
//!
//! Single free function that pattern-matches a [`Consequence`]
//! and writes the corresponding mutation. Quest outcomes, event
//! triggers, and choice effects all route through [`apply`].

use bevy::prelude::*;
use bevy_prng::WyRand;
use cordon_core::entity::faction::Faction;
use cordon_core::item::ItemInstance;
use cordon_core::primitive::Id;
use cordon_core::world::narrative::Consequence;

use super::messages::{
    DecisionRecorded, EndGameRequest, GiveNpcXpRequest, SpawnNpcRequest, StandingChanged,
    StartQuestRequest,
};
use super::refs::{PlayerRefs, QuestTx, SimRefs};
use crate::day::world_events::{EventOverrides, spawn_event_instance};

/// Apply a single consequence. Warnings rather than panics so
/// content typos don't crash the sim.
pub fn apply(
    consequence: &Consequence,
    players: &mut PlayerRefs,
    tx: &mut QuestTx,
    sim: &mut SimRefs,
    rng: &mut WyRand,
    faction_pool: &[Id<Faction>],
) {
    match consequence {
        Consequence::StandingChange { faction, delta } => {
            if let Some(standing) = players.standings.standing_mut(faction) {
                standing.apply(*delta);
                tx.standing_changed.write(StandingChanged {
                    faction: faction.clone(),
                    delta: *delta,
                });
            } else {
                warn!("StandingChange: unknown faction `{}`", faction.as_str());
            }
        }

        Consequence::GiveCredits(amount) => {
            players.identity.credits += *amount;
        }

        Consequence::TakeCredits(amount) => {
            players.identity.credits -= *amount;
        }

        Consequence::GiveItem(q) => {
            let Some(def) = sim.data.item(&q.item) else {
                warn!("GiveItem: unknown item `{}`", q.item.as_str());
                return;
            };
            let count = q.resolved_count();
            for _ in 0..count {
                players.stash.add_item(ItemInstance::new(def), q.scope);
            }
        }

        Consequence::TakeItem(q) => {
            let count = q.resolved_count();
            let mut removed = 0u32;
            for _ in 0..count {
                if players.stash.remove_first(&q.item, q.scope).is_none() {
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
            let Some(def) = sim.data.events.get(event) else {
                warn!("TriggerEvent: unknown event `{}`", event.as_str());
                return;
            };
            let overrides = EventOverrides {
                target_area: target_area.clone(),
                involved_factions: involved_factions.clone(),
                duration_days: *duration_days,
            };
            let instance = spawn_event_instance(def, faction_pool, sim.now.day, &overrides, rng);
            sim.events.push(instance);
        }

        Consequence::StartQuest(quest_id) => {
            tx.start_quest.write(StartQuestRequest {
                quest: quest_id.clone(),
            });
        }

        Consequence::UnlockUpgrade(upgrade) => {
            if !players.upgrades.upgrades.contains(upgrade) {
                players.upgrades.upgrades.push(upgrade.clone());
            }
        }

        Consequence::GiveIntel(intel_id) => {
            players.intel.grant(intel_id.clone(), sim.now.day);
        }

        Consequence::SpawnNpc { template, at } => {
            let Some(def) = sim.data.npc_template(template) else {
                warn!("SpawnNpc: unknown template `{}`", template.as_str());
                return;
            };
            if def.unique && sim.registry.is_alive(template) {
                info!(
                    "SpawnNpc: unique template `{}` already alive, skipping",
                    template.as_str()
                );
                return;
            }
            if !def.respawnable && sim.registry.is_permanently_dead(template) {
                info!(
                    "SpawnNpc: non-respawnable template `{}` is permanently dead, skipping",
                    template.as_str()
                );
                return;
            }
            tx.spawn_npc.write(SpawnNpcRequest {
                template: template.clone(),
                at: at.clone(),
                yarn_node: None,
                delivery_items: Vec::new(),
            });
        }

        Consequence::GivePlayerXp(xp) => {
            players.identity.add_xp(xp.value());
        }

        Consequence::GiveNpcXp { template, amount } => {
            if !sim.registry.is_alive(template) {
                warn!(
                    "GiveNpcXp: template `{}` is not alive, cannot grant {} xp",
                    template.as_str(),
                    amount.value(),
                );
                return;
            }
            tx.give_npc_xp.write(GiveNpcXpRequest {
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
            tx.end_game.write(EndGameRequest { cause: *cause });
        }

        Consequence::RecordDecision { decision, value } => {
            info!("RecordDecision: {} = `{}`", decision.as_str(), value);
            players.decisions.record(decision.clone(), value.clone());
            tx.decision_recorded.write(DecisionRecorded {
                decision: decision.clone(),
                value: value.clone(),
            });
        }

        Consequence::UnlockSupplier { template } => {
            if players.suppliers.is_unlocked(template) {
                info!(
                    "UnlockSupplier: `{}` already unlocked, skipping",
                    template.as_str()
                );
            } else {
                info!("UnlockSupplier: `{}` unlocked", template.as_str());
                players.suppliers.unlock(template.clone());
            }
        }
    }
}
