//! [`Consequence`] application.
//!
//! Single-entry mutator that walks a [`Consequence`] and writes
//! the corresponding change into world state. Quest outcomes,
//! event triggers, choice effects, and any future narrative
//! hook all go through [`apply`] so the behaviour of each
//! variant is defined in exactly one place.
//!
//! The applier borrows a [`WorldMut`] bundle of mutable
//! references. Side-effects that can't be performed through
//! borrows alone — starting a new quest, firing an event —
//! queue messages for downstream systems rather than mutating
//! the caller's world directly.

use bevy::prelude::*;
use bevy_prng::WyRand;
use cordon_core::entity::faction::Faction;
use cordon_core::entity::npc::NpcTemplate;
use cordon_core::entity::player::PlayerState;
use cordon_core::item::ItemInstance;
use cordon_core::primitive::{Experience, GameTime, Id};
use cordon_core::world::area::Area;
use cordon_core::world::narrative::{ActiveEvent, Consequence, Quest};
use cordon_data::catalog::GameData;

use super::registry::TemplateRegistry;
use crate::day::world_events::{EventOverrides, spawn_event_instance};

/// Live references the applier may mutate in place.
///
/// `rng` and `faction_pool` are threaded through so
/// consequences that need randomness (e.g.
/// [`TriggerEvent`](Consequence::TriggerEvent)) can build
/// realistic instances through the shared
/// [`spawn_event_instance`] helper instead of hardcoding
/// def-minimum values.
pub struct WorldMut<'a> {
    pub player: &'a mut PlayerState,
    pub events: &'a mut Vec<ActiveEvent>,
    pub data: &'a GameData,
    pub registry: &'a TemplateRegistry,
    pub now: GameTime,
    pub rng: &'a mut WyRand,
    pub faction_pool: &'a [Id<Faction>],
}

/// Message emitted by the applier whenever a consequence asks
/// for a new quest to be started outside of the regular trigger
/// flow. Consumed by the quest dispatcher system.
#[derive(Message, Debug, Clone)]
pub struct StartQuestRequest {
    pub quest: Id<Quest>,
}

/// Emitted by `SpawnNpc` consequences. A downstream Bevy system
/// (in cordon-bevy) consumes these to enqueue the visitor or
/// place the NPC in the zone, then registers the entity in
/// [`TemplateRegistry`].
#[derive(Message, Debug, Clone)]
pub struct SpawnNpcRequest {
    pub template: Id<NpcTemplate>,
    pub at: Option<Id<Area>>,
    /// When set, the spawned template NPC will dispatch this
    /// yarn node as its visitor payload when it arrives at the
    /// bunker. `None` for generic `SpawnNpc` consequences that
    /// drop the NPC into the world without a conversation.
    pub yarn_node: Option<String>,
}

/// Emitted by the dialogue bridge when a template NPC's
/// conversation completes. Consumed by a Bevy-layer system that
/// starts the return-travel leg: strips `QuestCritical`, attaches
/// `TravelingHome`, and builds a fresh 1-member squad that walks
/// the NPC back to its `SpawnOrigin`.
#[derive(Message, Debug, Clone)]
pub struct DismissTemplateNpc {
    pub entity: Entity,
    pub template: Id<NpcTemplate>,
}

/// Emitted by `GiveNpcXp` consequences. Consumed by a downstream
/// system that resolves the template to a live entity via
/// [`TemplateRegistry`] and mutates its [`Experience`] component.
#[derive(Message, Debug, Clone)]
pub struct GiveNpcXpRequest {
    pub template: Id<NpcTemplate>,
    pub amount: Experience,
}

/// Apply a single consequence.
///
/// Warnings rather than errors: quests are content-authored and
/// a typo in a reward item ID should not crash the sim. Missing
/// lookups log and skip.
pub fn apply(
    consequence: &Consequence,
    world: &mut WorldMut<'_>,
    start_quest: &mut MessageWriter<StartQuestRequest>,
    spawn_npc: &mut MessageWriter<SpawnNpcRequest>,
    give_npc_xp: &mut MessageWriter<GiveNpcXpRequest>,
) {
    match consequence {
        Consequence::StandingChange { faction, delta } => {
            if let Some(standing) = world.player.standing_mut(faction) {
                standing.apply(*delta);
            } else {
                warn!("StandingChange: unknown faction `{}`", faction.as_str());
            }
        }

        Consequence::GiveCredits(amount) => {
            world.player.credits += *amount;
        }

        Consequence::TakeCredits(amount) => {
            world.player.credits -= *amount;
        }

        Consequence::GiveItem(q) => {
            let Some(def) = world.data.item(&q.item) else {
                warn!("GiveItem: unknown item `{}`", q.item.as_str());
                return;
            };
            let count = q.resolved_count();
            for _ in 0..count {
                let instance = ItemInstance::new(def);
                if let Err(dropped) = world.player.add_item(instance, q.scope) {
                    warn!(
                        "GiveItem: stash full, dropped `{}` on the floor",
                        dropped.def_id.as_str()
                    );
                    break;
                }
            }
        }

        Consequence::TakeItem(q) => {
            let count = q.resolved_count();
            let mut removed = 0u32;
            for _ in 0..count {
                if world.player.remove_first(&q.item, q.scope).is_none() {
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
            let Some(def) = world.data.events.get(event) else {
                warn!("TriggerEvent: unknown event `{}`", event.as_str());
                return;
            };
            // Share the same instancing helper the day-cycle
            // roll uses so the two paths can never drift on
            // duration / faction / target-area randomness.
            // Any consequence-supplied override pins its field;
            // the rest falls through to the def-driven rng.
            let overrides = EventOverrides {
                target_area: target_area.clone(),
                involved_factions: involved_factions.clone(),
                duration_days: *duration_days,
            };
            let instance = spawn_event_instance(
                def,
                world.faction_pool,
                world.now.day,
                &overrides,
                world.rng,
            );
            world.events.push(instance);
        }

        Consequence::StartQuest(quest_id) => {
            start_quest.write(StartQuestRequest {
                quest: quest_id.clone(),
            });
        }

        Consequence::UnlockUpgrade(upgrade) => {
            if !world.player.upgrades.contains(upgrade) {
                world.player.upgrades.push(upgrade.clone());
            }
        }

        Consequence::SpawnNpc { template, at } => {
            let Some(def) = world.data.npc_template(template) else {
                warn!("SpawnNpc: unknown template `{}`", template.as_str());
                return;
            };
            if def.unique && world.registry.is_alive(template) {
                info!(
                    "SpawnNpc: unique template `{}` already alive, skipping",
                    template.as_str()
                );
                return;
            }
            if !def.respawnable && world.registry.is_permanently_dead(template) {
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
            world.player.add_xp(xp.value());
        }

        Consequence::GiveNpcXp { template, amount } => {
            if !world.registry.is_alive(template) {
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
            // `AreaStates` is a separate resource; routing
            // through the applier needs a dedicated message
            // channel that does not exist yet. The duration
            // override is included in the warning so the full
            // intent surfaces before the bridge is wired.
            let target = area
                .as_ref()
                .map(|a| a.as_str().to_string())
                .unwrap_or_else(|| "zone-wide".to_string());
            let lifetime = duration
                .map(|d| d.to_string())
                .unwrap_or_else(|| "permanent".to_string());
            warn!(
                "STUB CONSEQUENCE `danger_modifier` fired — no AreaStates bridge yet. \
                 Area `{target}` will not receive danger delta {delta} (lifetime {lifetime})."
            );
        }

        Consequence::PriceModifier {
            category,
            multiplier,
            duration,
        } => {
            // The trade loop is still a stub; no market
            // system to receive price shifts.
            let lifetime = duration
                .map(|d| d.to_string())
                .unwrap_or_else(|| "permanent".to_string());
            warn!(
                "STUB CONSEQUENCE `price_modifier` fired — no market system yet. \
                 Category {category:?} will not be multiplied by {multiplier} (lifetime {lifetime})."
            );
        }
    }
}
