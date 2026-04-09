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
use cordon_core::entity::player::PlayerState;
use cordon_core::item::ItemInstance;
use cordon_core::primitive::{GameTime, Id};
use cordon_core::world::narrative::{ActiveEvent, Consequence, Quest};
use cordon_data::catalog::GameData;

use crate::day::events::{EventOverrides, spawn_event_instance};

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
            // Visitor enqueueing lives in cordon-bevy's quest
            // bridge. The sim has no concept of "visitor
            // queue", so until the bridge observes a spawn
            // request this is a loud no-op. The `at` override
            // is captured in the warning so unwired calls
            // surface the intended spawn location too.
            let where_ = at
                .as_ref()
                .map(|a| a.as_str().to_string())
                .unwrap_or_else(|| "default".to_string());
            warn!(
                "STUB CONSEQUENCE `spawn_npc` fired — no visitor queue bridge yet. \
                 Template `{}` at `{}` will not appear in-game.",
                template.as_str(),
                where_,
            );
        }

        Consequence::GivePlayerXp(xp) => {
            world.player.add_xp(xp.value());
        }

        Consequence::GiveNpcXp { template, amount } => {
            // NPC XP grants need a template → entity resolver
            // in the behavior layer that does not exist yet.
            warn!(
                "STUB CONSEQUENCE `give_npc_xp` fired — no template→entity resolver yet. \
                 Template `{}` will not receive {} xp.",
                template.as_str(),
                amount.value(),
            );
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
