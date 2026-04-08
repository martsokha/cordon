//! The concrete glue between quest state and the dialogue runner.
//!
//! Two entry points:
//!
//! - [`enqueue_talk_visitors`] runs every frame in the bunker
//!   state and pushes a [`Visitor`] onto [`VisitorQueue`] for
//!   each active quest that has just entered a `Talk` stage.
//!   Idempotency is handled by a bridge-owned
//!   [`DialogueInFlight`] set — the bridge records which
//!   quests it has already dispatched so the same quest
//!   doesn't get enqueued twice per stage.
//! - [`on_dialogue_completed`] is a Bevy observer on
//!   `DialogueCompleted`. It finds the active quest whose
//!   dialogue just finished, copies any `$quest_*` Yarn
//!   variables into the quest's flag bag, calls
//!   [`engine::advance_after_talk`] with whatever
//!   `$quest_choice` the Yarn node wrote, and clears the
//!   quest from [`DialogueInFlight`] so the next `Talk`
//!   stage (if any) is free to enqueue a fresh visitor.

use std::collections::HashSet;

use bevy::prelude::*;
use bevy_yarnspinner::events::DialogueCompleted;
use bevy_yarnspinner::prelude::{DialogueRunner, YarnValue};
use cordon_core::entity::faction::Faction;
use cordon_core::primitive::Id;
use cordon_core::world::narrative::quest::{Quest, QuestStageKind};
use cordon_data::gamedata::GameDataResource;
use cordon_sim::plugin::prelude::{GameClock, QuestLog};
use cordon_sim::quest::engine::advance_after_talk;

use crate::bunker::{Visitor, VisitorQueue, VisitorState};

/// Bridge-owned idempotency set. Holds the IDs of quests that
/// have been enqueued on the visitor queue but whose dialogue
/// has not yet completed. Keeps [`enqueue_talk_visitors`]
/// idempotent across frames without leaking state into
/// [`ActiveQuest`](cordon_sim::quest::ActiveQuest).
#[derive(Resource, Debug, Default)]
pub struct DialogueInFlight(pub HashSet<Id<Quest>>);

/// Yarn variable name the bridge treats as "the player's choice"
/// when deciding which `Talk` branch to follow. A Yarn node
/// writes this inside the option the player picked:
///
/// ```yarn
/// -> I'll help.
///     <<set $quest_choice = "accept">>
/// ```
const CHOICE_VAR: &str = "$quest_choice";

/// Prefix used to filter which Yarn variables get captured back
/// into the quest flag bag. Everything that matches is copied
/// verbatim — later stages can then branch on them via
/// [`ObjectiveCondition::QuestFlag`](cordon_core::world::narrative::consequence::ObjectiveCondition::QuestFlag).
const FLAG_PREFIX: &str = "$quest_";

/// For each active quest currently parked on a `Talk` stage
/// that hasn't been enqueued yet, push a [`Visitor`] onto the
/// bunker's [`VisitorQueue`] and record the quest ID in
/// [`DialogueInFlight`]. Runs every frame — the in-flight set
/// keeps it idempotent.
///
/// Because the in-flight tag lives outside `ActiveQuest`, a
/// read-only borrow of the quest log is enough here. The only
/// mutation is on the bridge-local [`DialogueInFlight`] and
/// the visitor queue.
pub fn enqueue_talk_visitors(
    log: Res<QuestLog>,
    data: Res<GameDataResource>,
    mut queue: ResMut<VisitorQueue>,
    mut in_flight: ResMut<DialogueInFlight>,
) {
    let catalog = &data.0;
    for active in &log.active {
        if in_flight.0.contains(&active.def_id) {
            continue;
        }
        let Some(def) = catalog.quests.get(&active.def_id) else {
            continue;
        };
        let Some(stage) = def.stage(&active.current_stage) else {
            continue;
        };
        let QuestStageKind::Talk {
            npc: stage_npc,
            yarn_node,
            ..
        } = &stage.kind
        else {
            continue;
        };

        // Narrator lines (no giver on the stage or the quest)
        // can't be delivered by the visitor queue. Skip for now
        // and log so the omission is visible in authoring.
        let npc_template = stage_npc.as_ref().or(def.giver.as_ref());
        let Some(template) = npc_template else {
            warn!(
                "quest `{}` stage `{}` is Talk with no NPC — narrator-only stages are not yet supported",
                def.id.as_str(),
                active.current_stage.as_str()
            );
            continue;
        };

        let faction = def
            .giver_faction
            .clone()
            .unwrap_or_else(|| Id::<Faction>::new("drifters"));

        queue.0.push_back(Visitor {
            display_name: template.as_str().to_string(),
            faction,
            yarn_node: yarn_node.clone(),
            quest: Some(active.def_id.clone()),
        });
        in_flight.0.insert(active.def_id.clone());
        info!(
            "quest `{}` enqueued visitor `{}` for Talk stage `{}`",
            active.def_id.as_str(),
            template.as_str(),
            active.current_stage.as_str()
        );
    }
}

/// Bevy observer: `DialogueCompleted` fires once Yarn has
/// finished running the node the player was talking through.
///
/// Identifies the relevant quest via the visitor the player is
/// currently talking to: the [`VisitorState::Inside`] payload
/// still holds the [`Visitor`] while this observer runs,
/// because the dismissal system hasn't ticked yet. The
/// visitor's `quest` field (set by
/// [`enqueue_talk_visitors`]) points straight at the active
/// quest we need to advance. This is more robust than
/// scanning `QuestLog` for an "awaiting dialogue" flag — it
/// cannot confuse two parallel quests.
pub fn on_dialogue_completed(
    _event: On<DialogueCompleted>,
    mut log: ResMut<QuestLog>,
    data: Res<GameDataResource>,
    clock: Res<GameClock>,
    visitor_state: Res<VisitorState>,
    mut in_flight: ResMut<DialogueInFlight>,
    runner_q: Query<&DialogueRunner>,
) {
    let VisitorState::Inside { visitor, .. } = &*visitor_state else {
        // Non-quest dialogue (or dialogue ended outside the
        // visitor flow). Nothing to do.
        return;
    };
    let Some(quest_id) = visitor.quest.clone() else {
        // Ambient visitor — no quest to advance.
        return;
    };

    // Regardless of what happens below, the dialogue round
    // is over for this quest — clear the in-flight latch so
    // a subsequent `Talk` stage can enqueue a fresh visitor.
    in_flight.0.remove(&quest_id);

    let Ok(runner) = runner_q.single() else {
        warn!("DialogueCompleted: no dialogue runner entity found");
        return;
    };
    let variables = runner.variable_storage().variables();

    // Copy every $quest_* variable into the active quest's
    // flag bag, overwriting any previous value with the same
    // key. This includes `$quest_choice` itself so later
    // conditions can read it via `QuestFlag` too.
    let mut captured_choice: Option<String> = None;
    if let Some(active) = log.active_instance_mut(&quest_id) {
        for (name, value) in variables {
            if !name.starts_with(FLAG_PREFIX) {
                continue;
            }
            if name == CHOICE_VAR
                && let YarnValue::String(s) = &value
            {
                captured_choice = Some(s.clone());
            }
            active.flags.insert(name, value);
        }
    } else {
        warn!(
            "DialogueCompleted: visitor tagged quest `{}` not in active log",
            quest_id.as_str()
        );
        return;
    }

    advance_after_talk(
        &mut log,
        &data.0,
        &quest_id,
        captured_choice.as_deref(),
        clock.0,
    );
}
