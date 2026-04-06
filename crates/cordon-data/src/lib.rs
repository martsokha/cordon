#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

/// Game data catalog: the read-only database of all definitions.
pub mod catalog;

/// Bevy plugin for loading game data from JSON assets.
pub mod gamedata;
