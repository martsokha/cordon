#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

/// Game data catalog: the read-only database of all definitions.
pub mod catalog;

/// Loot tables: per-sector weighted drop tables.
pub mod loot;
