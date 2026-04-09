#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]
// Bevy systems naturally have many resource params and complex Query
// types — these lints fire on idiomatic Bevy code, so they're allowed
// crate-wide rather than per-system.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

/// Game data catalog: the read-only database of all definitions.
/// Includes `GameData::validate` for post-load integrity checks.
pub mod catalog;

/// Bevy plugin for loading game data from JSON assets.
pub mod gamedata;
