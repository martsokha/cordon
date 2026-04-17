//! Runtime quest system.
//!
//! - [`state`] — [`ActiveQuest`], [`CompletedQuest`], [`QuestLog`]
//! - [`condition`] — recursive [`ObjectiveCondition`] evaluator
//! - [`consequence`] — [`Consequence`] applier
//! - [`context`] — unified [`QuestCtx`] system parameter
//! - [`messages`] — all quest-related messages
//! - [`dispatch`] — trigger dispatch (game start, day, event, condition)
//! - [`drive`] — per-frame stage driving and talk completion
//! - [`death`] — template NPC death handling
//! - [`registry`] — template NPC tracking
//! - [`travel`] — arrival/departure detection

pub mod condition;
pub mod consequence;
pub mod context;
pub mod death;
pub mod dispatch;
pub mod drive;
pub mod messages;
pub mod registry;
pub mod state;
pub mod travel;

use bevy::prelude::*;

pub use self::messages::{
    DismissTemplateNpc, GiveNpcXpRequest, QuestFinished, QuestStarted, QuestUpdated,
    SpawnNpcRequest, StandingChanged, StartQuestRequest, TalkCompleted,
};
pub use self::registry::TemplateRegistry;
pub use self::state::{ActiveQuest, CompletedQuest, QuestLog};
pub use self::travel::{BunkerArrival, HomeArrival};
use crate::day::DayRolled;
use crate::plugin::SimSet;
use crate::resources::GameClock;

pub struct QuestPlugin;

impl Plugin for QuestPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<QuestLog>();
        app.init_resource::<TemplateRegistry>();
        app.add_message::<StartQuestRequest>();
        app.add_message::<SpawnNpcRequest>();
        app.add_message::<GiveNpcXpRequest>();
        app.add_message::<StandingChanged>();
        app.add_message::<QuestStarted>();
        app.add_message::<QuestUpdated>();
        app.add_message::<QuestFinished>();
        app.add_message::<TalkCompleted>();
        app.add_message::<BunkerArrival>();
        app.add_message::<HomeArrival>();
        app.add_message::<DismissTemplateNpc>();

        app.add_systems(
            Update,
            dispatch::dispatch_on_game_start.run_if(resource_added::<GameClock>),
        );

        app.add_systems(
            Update,
            (
                dispatch::dispatch_on_day.run_if(on_message::<DayRolled>),
                dispatch::dispatch_on_event,
                dispatch::dispatch_on_condition,
                drive::handle_talk_completed,
                drive::drive_active_quests,
                dispatch::process_start_quest_requests,
                death::fail_talk_on_template_death,
            )
                .chain()
                .in_set(SimSet::Cleanup)
                .run_if(resource_exists::<GameClock>),
        );

        app.add_systems(
            Update,
            (
                travel::detect_bunker_arrival,
                travel::detect_home_arrival,
                travel::prune_despawned_templates,
            ),
        );
    }
}
