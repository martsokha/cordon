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
use cordon_core::entity::player::PlayerState;
use cordon_core::item::ItemInstance;
use cordon_core::primitive::{GameTime, Id};
use cordon_core::world::event::ActiveEvent;
use cordon_core::world::narrative::consequence::Consequence;
use cordon_core::world::narrative::quest::Quest;
use cordon_data::catalog::GameData;

/// Live references the applier may mutate in place.
pub struct WorldMut<'a> {
    pub player: &'a mut PlayerState,
    pub events: &'a mut Vec<ActiveEvent>,
    pub data: &'a GameData,
    pub now: GameTime,
}

/// Message emitted by the applier whenever a consequence asks
/// for a new quest to be started outside of the regular trigger
/// flow. Consumed by the quest dispatcher system.
#[derive(Message, Debug, Clone)]
pub struct StartQuestRequest {
    pub quest: Id<Quest>,
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

        Consequence::GiveItem { item, count, scope } => {
            let Some(def) = world.data.item(item) else {
                warn!("GiveItem: unknown item `{}`", item.as_str());
                return;
            };
            for _ in 0..*count {
                let instance = ItemInstance::new(def);
                if let Err(dropped) = world.player.add_item(instance, *scope) {
                    warn!(
                        "GiveItem: stash full, dropped `{}` on the floor",
                        dropped.def_id.as_str()
                    );
                    break;
                }
            }
        }

        Consequence::TakeItem { item, count, scope } => {
            let mut removed = 0u32;
            for _ in 0..*count {
                if world.player.remove_first(item, *scope).is_none() {
                    break;
                }
                removed += 1;
            }
            if removed < *count {
                warn!(
                    "TakeItem: wanted {count}× `{}` in scope {:?}, only removed {removed}",
                    item.as_str(),
                    scope
                );
            }
        }

        Consequence::TriggerEvent(event_id) => {
            let Some(def) = world.data.events.get(event_id) else {
                warn!("TriggerEvent: unknown event `{}`", event_id.as_str());
                return;
            };
            // The day-cycle system owns the normal event roll; a
            // consequence-driven fire bypasses probability and
            // duration rolling by using the def's minimum values
            // directly. Designers wanting randomness can express
            // it via multiple consequence variants.
            world.events.push(ActiveEvent {
                def_id: def.id.clone(),
                day_started: world.now.day,
                duration_days: def.min_duration,
                involved_factions: def.involved_factions.clone(),
                target_area: def.target_areas.first().cloned(),
            });
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

        Consequence::SpawnNpc(template) => {
            // Visitor enqueueing lives in cordon-bevy's quest
            // bridge. The sim has no concept of "visitor
            // queue", so until the bridge observes a spawn
            // request this is a loud no-op.
            warn!(
                "STUB CONSEQUENCE `spawn_npc` fired — no visitor queue bridge yet. \
                 Template `{}` will not appear in-game.",
                template.as_str()
            );
        }

        Consequence::GivePlayerXp(amount) => {
            world.player.add_xp(*amount);
        }

        Consequence::GiveNpcXp { template, amount } => {
            // NPC XP grants need a template → entity resolver
            // in the behavior layer that does not exist yet.
            warn!(
                "STUB CONSEQUENCE `give_npc_xp` fired — no template→entity resolver yet. \
                 Template `{}` will not receive {amount} xp.",
                template.as_str()
            );
        }

        Consequence::DangerModifier { area, delta } => {
            // `AreaStates` is a separate resource; routing
            // through the applier needs a dedicated message
            // channel that does not exist yet.
            let target = area
                .as_ref()
                .map(|a| a.as_str().to_string())
                .unwrap_or_else(|| "zone-wide".to_string());
            warn!(
                "STUB CONSEQUENCE `danger_modifier` fired — no AreaStates bridge yet. \
                 Area `{target}` will not receive danger delta {delta}."
            );
        }

        Consequence::PriceModifier {
            category,
            multiplier,
        } => {
            // The trade loop is still a stub; no market
            // system to receive price shifts.
            warn!(
                "STUB CONSEQUENCE `price_modifier` fired — no market system yet. \
                 Category {category:?} will not be multiplied by {multiplier}."
            );
        }
    }
}
