#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

/// Game data catalog: the read-only database of all definitions.
pub mod catalog;

/// Asset loading from the filesystem.
pub mod loader;

/// Loot tables: per-area weighted drop tables.
pub mod loot;
