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
//! - [`validate`] — load-time catalog checks.

mod dispatch;
mod drive;
mod talk;
mod validate;

pub use self::dispatch::{
    QuestDispatchCtx, dispatch_on_condition, dispatch_on_day, dispatch_on_event,
    dispatch_on_game_start, process_start_quest_requests,
};
pub use self::drive::{QuestEngineCtx, drive_active_quests};
pub use self::talk::advance_after_talk;
pub use self::validate::validate_catalog;
