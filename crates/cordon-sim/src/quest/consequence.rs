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

        Consequence::GiveItem { item, scope } => {
            let Some(def) = world.data.item(item) else {
                warn!("GiveItem: unknown item `{}`", item.as_str());
                return;
            };
            let instance = ItemInstance::new(def);
            if let Err(dropped) = world.player.add_item(instance, *scope) {
                warn!(
                    "GiveItem: stash full, dropped `{}` on the floor",
                    dropped.def_id.as_str()
                );
            }
        }

        Consequence::TakeItem { item, scope } => {
            if world.player.remove_first(item, *scope).is_none() {
                warn!(
                    "TakeItem: player has no `{}` in scope {:?}",
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
            // Real visitor enqueueing lives in cordon-bevy — the
            // sim has no concept of "visitor queue". The bridge
            // listens for the log entry and pushes a visitor with
            // the right yarn node. We emit a warning here so the
            // lack of a bridge is visible in tests.
            info!("SpawnNpc requested for template `{}`", template.as_str());
        }

        Consequence::GivePlayerXp(amount) => {
            world.player.add_xp(*amount);
        }

        Consequence::GiveNpcXp { template, amount } => {
            // NPC XP grants need a resolved entity, which lives
            // in the behavior layer. Phase 3 stops at logging;
            // Phase 4 will wire this through a message the
            // cordon-bevy NPC layer observes.
            info!(
                "GiveNpcXp: template `{}` += {amount} xp (not yet wired)",
                template.as_str()
            );
        }

        Consequence::DangerModifier { area: _, delta: _ } => {
            // AreaStates is a separate resource; applying this
            // through the Consequence path requires either
            // borrowing it too (blows up the WorldMut arg list)
            // or routing through a dedicated message. We'll use
            // a message in Phase 4 once the first quest actually
            // needs it.
            info!("DangerModifier consequence not yet wired");
        }

        Consequence::PriceModifier {
            category: _,
            multiplier: _,
        } => {
            // Market modifiers have no system to receive them
            // yet — the trade loop is still a stub. Logged so
            // quest authoring can reference the consequence
            // without crashing.
            info!("PriceModifier consequence not yet wired");
        }
    }
}
