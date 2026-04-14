//! Talk-stage completion bridge.
//!
//! [`advance_after_talk`] is the one engine entry point that
//! does not live on a [`QuestDispatchCtx`](super::QuestDispatchCtx)
//! or [`QuestEngineCtx`](super::QuestEngineCtx) — it's called
//! from cordon-bevy's Yarn bridge, which has no SystemParam
//! bundle on its side. The bridge assembles the required
//! references by hand and calls this function directly.

use cordon_core::primitive::{GameTime, Id};
use cordon_core::world::narrative::{Quest, QuestStageKind};
use cordon_data::catalog::GameData;

use super::super::condition::WorldView;
use super::super::registry::TemplateRegistry;
use super::super::state::QuestLog;

/// After a Yarn dialogue tied to a `Talk` stage finishes, jump
/// to the first eligible branch whose `choice` matches the
/// supplied value, or to the stage's `fallback` if nothing
/// matches. Call this from the cordon-bevy dialogue bridge.
///
/// A branch is *eligible* when its
/// [`requires`](cordon_core::world::narrative::TalkBranch::requires)
/// guard is absent or evaluates true against the current world
/// view. Inelegible branches are skipped during selection, so
/// authors can express "you can only take this branch if you
/// also carry the medkit" without teaching Yarn the rule.
#[allow(clippy::too_many_arguments)]
pub fn advance_after_talk(
    log: &mut QuestLog,
    data: &GameData,
    player: &cordon_core::entity::player::PlayerState,
    events: &[cordon_core::world::narrative::ActiveEvent],
    registry: &TemplateRegistry,
    quest: &Id<Quest>,
    choice: Option<&str>,
    now: GameTime,
) {
    let Some(active) = log.active_instance(quest) else {
        return;
    };
    let Some(def) = data.quests.get(&active.def_id) else {
        return;
    };
    let Some(stage) = def.stage(&active.current_stage) else {
        return;
    };
    let QuestStageKind::Talk(talk) = &stage.kind else {
        return;
    };
    // Build a view with the stage clock so any guards that
    // reference `Wait` or other stage-scoped checks still work.
    let view = WorldView {
        player,
        events,
        quests: log,
        registry,
        now,
        stage_started_at: Some(active.stage_started_at),
    };
    let next = match choice {
        Some(c) => talk
            .branches
            .iter()
            .filter(|b| {
                b.requires
                    .as_ref()
                    .map(|cond| view.evaluate(cond))
                    .unwrap_or(true)
            })
            .find(|b| b.choice == c)
            .map(|b| b.next_stage.clone())
            .unwrap_or_else(|| talk.fallback.clone()),
        None => talk.fallback.clone(),
    };
    // Mutable re-borrow now that evaluation is done.
    if let Some(active) = log.active_instance_mut(quest) {
        active.advance_to(next, now);
    }
}
