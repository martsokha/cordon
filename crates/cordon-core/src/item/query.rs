//! Item-query struct shared by object conditions and consequences.
//!
//! [`ItemQuery`] packages the three pieces of information every
//! "which items does this touch" call needs: *what* item def,
//! *how many*, and in *which* stash scope. The same struct shows
//! up in [`ObjectiveCondition::HaveItem`](crate::world::narrative::ObjectiveCondition::HaveItem),
//! [`Consequence::GiveItem`](crate::world::narrative::Consequence::GiveItem),
//! and [`Consequence::TakeItem`](crate::world::narrative::Consequence::TakeItem)
//! — one shape, one invariant, one place to evolve.

use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use super::{Item, StashScope};
use crate::primitive::Id;

/// A scoped query for an item in the player's stash.
///
/// `count` is [`None`] for the common "one copy" case — authors
/// omit the field entirely and the type guarantees it can never
/// be zero. Read through [`resolved_count`](Self::resolved_count)
/// to get the effective value.
///
/// `scope` is [`StashScope::Main`] by default via `#[serde(default)]`,
/// matching the author intuition that unmarked item operations
/// touch the main stash.
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
pub struct ItemQuery {
    /// Which item definition this query targets.
    pub item: Id<Item>,
    /// Number of copies. `None` means one — the default — and
    /// the `NonZeroU32` niche makes "zero copies" unrepresentable
    /// without spending an extra discriminant byte.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<NonZeroU32>,
    /// Which stash scope the query applies to. Defaults to
    /// [`StashScope::Main`] when omitted.
    #[serde(default)]
    pub scope: StashScope,
}

impl ItemQuery {
    /// Shorthand for a one-copy, main-scope query. Keeps test
    /// and programmatic call sites readable.
    pub fn one(item: Id<Item>) -> Self {
        Self {
            item,
            count: None,
            scope: StashScope::Main,
        }
    }

    /// The effective count — unwraps [`count`](Self::count) to
    /// `1` when unset. Call this at every evaluator / applier
    /// site instead of reading the field directly, so the
    /// default stays in one place.
    pub fn resolved_count(&self) -> u32 {
        self.count.map(NonZeroU32::get).unwrap_or(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omitted_fields_default_to_one_main() {
        let json = r#"{ "item": "medkit" }"#;
        let q: ItemQuery = serde_json::from_str(json).unwrap();
        assert_eq!(q.resolved_count(), 1);
        assert_eq!(q.scope, StashScope::Main);
        assert!(q.count.is_none());
    }

    #[test]
    fn explicit_count_and_scope_parse() {
        let json = r#"{ "item": "bandage", "count": 5, "scope": "hidden" }"#;
        let q: ItemQuery = serde_json::from_str(json).unwrap();
        assert_eq!(q.resolved_count(), 5);
        assert_eq!(q.scope, StashScope::Hidden);
    }

    #[test]
    fn zero_count_is_rejected() {
        let json = r#"{ "item": "bandage", "count": 0 }"#;
        let err = serde_json::from_str::<ItemQuery>(json).unwrap_err();
        assert!(err.to_string().contains("zero"));
    }

    #[test]
    fn omit_count_roundtrip_stays_compact() {
        let q = ItemQuery::one(Id::<Item>::new("medkit"));
        let json = serde_json::to_string(&q).unwrap();
        // skip_serializing_if strips the null, scope still shows
        // because StashScope has no skip_if attribute.
        assert!(!json.contains("count"));
    }
}
