//! Which stash a quest operation targets.
//!
//! Quests address player storage through a [`StashScope`] rather
//! than touching [`Stash`](super::Stash) containers directly, so a
//! smuggling quest can declare "this reward goes into hidden
//! storage" in data without any bespoke code. The sim layer
//! resolves the scope against the player's two containers when it
//! evaluates conditions or applies consequences.
//!
//! `StashScope` is purely a quest-facing vocabulary. It is not
//! coupled to any raid / inspection mechanic — a future raid
//! system would read the same scope enum to decide what is
//! visible, but that system does not exist yet and is out of
//! scope here.

use serde::{Deserialize, Serialize};

/// Which stash(es) a quest condition or consequence targets.
///
/// The default is [`Main`](StashScope::Main): rewards and
/// consumable drops land in the player's visible bunker
/// stash unless a quest explicitly requests otherwise. This
/// matches intuition ("I got paid, my stuff shows up where I
/// keep my stuff") and avoids a surprising "where did that
/// reward go?" moment when only hidden-stash slots are free.
///
/// # Read vs. write semantics
///
/// - **On reads** ([`has_item`](super::super::entity::player::PlayerState::has_item),
///   [`item_count`](super::super::entity::player::PlayerState::item_count)):
///   `Any` counts items across both stashes. Explicit `Main`
///   or `Hidden` narrows the search.
/// - **On writes** ([`add_item`](super::super::entity::player::PlayerState::add_item)):
///   `Any` prefers main with hidden as overflow — useful for
///   "give the player this, but spill to hidden if main is
///   full" fallbacks. Explicit `Main` or `Hidden` errors
///   instead of spilling when the target stash is full.
/// - **On removes** ([`remove_first`](super::super::entity::player::PlayerState::remove_first)):
///   `Any` searches main first. Explicit scopes narrow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StashScope {
    /// The main visible bunker stash. Default for
    /// quest-authored rewards and deductions.
    #[default]
    Main,
    /// The hidden stash. Requested explicitly by smuggling-
    /// themed quests that want a reward to survive raids.
    Hidden,
    /// Both stashes. On reads, the union; on writes, main-
    /// preferred with hidden as overflow; on removes, main-
    /// first search. Use this when the quest genuinely does
    /// not care which stash the operation targets — rare in
    /// practice, explicit [`Main`](Self::Main) is usually
    /// clearer.
    Any,
}
