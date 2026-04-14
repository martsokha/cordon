//! Quest state transitions driven by world state.
//!
//! Split by capability:
//!
//! - [`dispatch`] — [`QuestDispatchCtx`] and the read-only
//!   trigger/start pipeline riding on it.
//! - [`drive`] — [`QuestEngineCtx`] and the mutable per-frame
//!   driver that applies consequences on outcome.
//! - [`talk`] — [`advance_after_talk`], the cordon-sim /
//!   cordon-bevy boundary call the Yarn bridge reaches back
//!   into when a `Talk` stage finishes.
//!
//! Catalog validation used to live here as a one-shot system;
//! it now runs inline during [`assemble_game_data`](cordon_data::gamedata)
//! as a method on [`GameData`](cordon_data::catalog::GameData).

mod death;
mod dispatch;
mod drive;
mod talk;

pub use self::death::fail_talk_on_template_death;
pub use self::dispatch::{
    QuestDispatchCtx, dispatch_on_condition, dispatch_on_day, dispatch_on_event,
    dispatch_on_game_start, process_start_quest_requests,
};
pub use self::drive::{QuestEngineCtx, drive_active_quests};
pub use self::talk::advance_after_talk;
