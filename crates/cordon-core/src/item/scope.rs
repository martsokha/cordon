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
/// The default is [`Any`](StashScope::Any) — most quests don't
/// care which container an item lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StashScope {
    /// The main visible bunker stash.
    Main,
    /// The hidden stash.
    Hidden,
    /// Either stash. Reads walk both; writes go to main first
    /// with hidden as overflow.
    #[default]
    Any,
}
