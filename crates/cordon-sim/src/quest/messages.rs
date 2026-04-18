//! All quest-related messages in one place.

use bevy::prelude::*;
use cordon_core::entity::faction::Faction;
use cordon_core::entity::npc::NpcTemplate;
use cordon_core::primitive::{Experience, Id, RelationDelta};
use cordon_core::world::area::Area;
use cordon_core::world::narrative::Quest;

/// Start a quest outside the regular trigger flow.
#[derive(Message, Debug, Clone)]
pub struct StartQuestRequest {
    pub quest: Id<Quest>,
}

/// Spawn a template NPC into the world.
#[derive(Message, Debug, Clone)]
pub struct SpawnNpcRequest {
    pub template: Id<NpcTemplate>,
    pub at: Option<Id<Area>>,
    pub yarn_node: Option<String>,
}

/// Dismiss a template NPC after dialogue (start return travel).
#[derive(Message, Debug, Clone)]
pub struct DismissTemplateNpc {
    pub entity: Entity,
    pub template: Id<NpcTemplate>,
}

/// Grant XP to a template NPC.
#[derive(Message, Debug, Clone)]
pub struct GiveNpcXpRequest {
    pub template: Id<NpcTemplate>,
    pub amount: Experience,
}

/// A faction standing changed via consequence.
#[derive(Message, Debug, Clone)]
pub struct StandingChanged {
    pub faction: Id<Faction>,
    pub delta: RelationDelta,
}

/// A quest was started.
#[derive(Message, Debug, Clone)]
pub struct QuestStarted {
    pub quest: Id<Quest>,
}

/// A quest advanced to a new stage (objective met, branch
/// resolved, talk completed).
#[derive(Message, Debug, Clone)]
pub struct QuestUpdated {
    pub quest: Id<Quest>,
}

/// A quest reached its outcome stage and completed.
#[derive(Message, Debug, Clone)]
pub struct QuestFinished {
    pub quest: Id<Quest>,
    pub success: bool,
}

/// A Talk stage's dialogue completed. Emitted by the cordon-app
/// Yarn bridge after copying flags; consumed by the drive system
/// to advance the quest stage.
#[derive(Message, Debug, Clone)]
pub struct TalkCompleted {
    pub quest: Id<Quest>,
    pub choice: Option<String>,
}
