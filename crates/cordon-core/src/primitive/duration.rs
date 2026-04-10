//! A duration value in game-minutes.
//!
//! One type for every "how long does this last" question in the
//! game — timed item effects, consumable use-times, quest time
//! limits, modifier lifetimes, periodic triggers. The minute is
//! the smallest unit the game speaks in; anything shorter would
//! only matter to animation code, which tracks its own clock.
//!
//! # JSON shape
//!
//! Authors can write a [`Duration`] as any of:
//!
//! ```json
//! "instant"          // zero minutes, also accepted as bare 0
//! 5                  // bare integer = minutes
//! { "mins": 30 }     // explicit minutes
//! { "hours": 3 }     // 3 × 60 minutes
//! { "days": 2 }      // 2 × 24 × 60 minutes
//! ```
//!
//! The bare-int form is the canonical compact shape. The tagged
//! forms exist for readability when authoring long durations —
//! `{ "days": 3 }` is kinder to reviewers than `4320`.

use std::num::NonZeroU32;

use serde::de::{Deserializer, Error as DeError, Unexpected};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

const MINUTES_PER_HOUR: u32 = 60;
const MINUTES_PER_DAY: u32 = 24 * MINUTES_PER_HOUR;

/// A game duration expressed in whole minutes.
///
/// Wraps an `Option<NonZeroU32>` — `None` means instant (zero
/// minutes), `Some(n)` means `n` minutes. The `Option` is the
/// same size as a `u32` thanks to [`NonZeroU32`]'s niche
/// optimization, so a `Duration` is eight bytes including the
/// discriminant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Duration(Option<NonZeroU32>);

impl Duration {
    /// An instant duration (zero minutes).
    pub const INSTANT: Self = Self(None);

    /// Create a duration from a raw minute count. Returns
    /// [`INSTANT`](Self::INSTANT) for `0`.
    pub const fn from_minutes(minutes: u32) -> Self {
        Self(NonZeroU32::new(minutes))
    }

    /// Create a duration from a whole-hour count. Saturates at
    /// `u32::MAX` minutes for absurd inputs.
    pub const fn from_hours(hours: u32) -> Self {
        Self::from_minutes(hours.saturating_mul(MINUTES_PER_HOUR))
    }

    /// Create a duration from a whole-day count. Saturates at
    /// `u32::MAX` minutes for absurd inputs.
    pub const fn from_days(days: u32) -> Self {
        Self::from_minutes(days.saturating_mul(MINUTES_PER_DAY))
    }

    /// Total minutes in this duration. `0` for instant.
    pub const fn minutes(self) -> u32 {
        match self.0 {
            Some(n) => n.get(),
            None => 0,
        }
    }

    /// Whether this duration is instant (zero minutes).
    pub const fn is_instant(self) -> bool {
        self.0.is_none()
    }
}

impl From<u32> for Duration {
    fn from(minutes: u32) -> Self {
        Self::from_minutes(minutes)
    }
}

impl std::fmt::Display for Duration {
    /// Prints the largest round unit: `"instant"`, `"3d"`, `"5h"`,
    /// `"42m"`. Falls back to minutes when the value doesn't
    /// divide cleanly.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Some(n) = self.0 else {
            return f.write_str("instant");
        };
        let m = n.get();
        if m % MINUTES_PER_DAY == 0 {
            write!(f, "{}d", m / MINUTES_PER_DAY)
        } else if m % MINUTES_PER_HOUR == 0 {
            write!(f, "{}h", m / MINUTES_PER_HOUR)
        } else {
            write!(f, "{}m", m)
        }
    }
}

// Custom serde: accept the multi-shape authoring DSL, emit the
// compact bare-int form on write so `serde_json::to_string`
// round-trips remain stable.

impl Serialize for Duration {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u32(self.minutes())
    }
}

/// Deserialization shim. Accepts any of:
///
/// - `"instant"` — zero minutes
/// - bare integer — minutes
/// - `{ "mins": N }` / `{ "hours": N }` / `{ "days": N }`
///
/// The `untagged` enum tries each variant in order and keeps the
/// first that parses. Bare int wins over the string because serde
/// tries `u32` before `String` for ambiguous JSON numbers.
#[derive(Deserialize)]
#[serde(untagged)]
enum DurationRepr {
    Raw(u32),
    Tagged(TaggedDuration),
    Keyword(String),
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum TaggedDuration {
    Mins(u32),
    Hours(u32),
    Days(u32),
}

impl<'de> Deserialize<'de> for Duration {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        match DurationRepr::deserialize(d)? {
            DurationRepr::Raw(m) => Ok(Self::from_minutes(m)),
            DurationRepr::Tagged(TaggedDuration::Mins(m)) => Ok(Self::from_minutes(m)),
            DurationRepr::Tagged(TaggedDuration::Hours(h)) => Ok(Self::from_hours(h)),
            DurationRepr::Tagged(TaggedDuration::Days(d)) => Ok(Self::from_days(d)),
            DurationRepr::Keyword(s) if s == "instant" => Ok(Self::INSTANT),
            DurationRepr::Keyword(s) => Err(DeError::invalid_value(
                Unexpected::Str(&s),
                &"\"instant\", a bare integer (minutes), or { mins | hours | days: N }",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_minutes_zero_is_instant() {
        assert_eq!(Duration::from_minutes(0), Duration::INSTANT);
        assert!(Duration::from_minutes(0).is_instant());
    }

    #[test]
    fn hour_and_day_constructors() {
        assert_eq!(Duration::from_hours(2).minutes(), 120);
        assert_eq!(Duration::from_days(3).minutes(), 3 * 24 * 60);
    }

    #[test]
    fn display_picks_largest_round_unit() {
        assert_eq!(Duration::INSTANT.to_string(), "instant");
        assert_eq!(Duration::from_minutes(42).to_string(), "42m");
        assert_eq!(Duration::from_minutes(120).to_string(), "2h");
        assert_eq!(Duration::from_minutes(60 * 24 * 3).to_string(), "3d");
        // Non-divisible values fall through to minutes.
        assert_eq!(Duration::from_minutes(61).to_string(), "61m");
    }

    #[test]
    fn deserialize_keyword_instant() {
        let d: Duration = serde_json::from_str(r#""instant""#).unwrap();
        assert_eq!(d, Duration::INSTANT);
    }

    #[test]
    fn deserialize_bare_int_is_minutes() {
        let d: Duration = serde_json::from_str("5").unwrap();
        assert_eq!(d.minutes(), 5);
    }

    #[test]
    fn deserialize_bare_zero_normalizes_to_instant() {
        let d: Duration = serde_json::from_str("0").unwrap();
        assert_eq!(d, Duration::INSTANT);
    }

    #[test]
    fn deserialize_tagged_mins_hours_days() {
        let m: Duration = serde_json::from_str(r#"{ "mins": 30 }"#).unwrap();
        assert_eq!(m.minutes(), 30);

        let h: Duration = serde_json::from_str(r#"{ "hours": 3 }"#).unwrap();
        assert_eq!(h.minutes(), 180);

        let d: Duration = serde_json::from_str(r#"{ "days": 2 }"#).unwrap();
        assert_eq!(d.minutes(), 2 * 24 * 60);
    }

    #[test]
    fn deserialize_unknown_keyword_errors() {
        let err = serde_json::from_str::<Duration>(r#""forever""#).unwrap_err();
        assert!(err.to_string().contains("instant"));
    }

    #[test]
    fn serialize_emits_bare_minutes() {
        let d = Duration::from_hours(2);
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(json, "120");
    }
}
