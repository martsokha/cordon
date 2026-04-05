//! Item definitions, effects, calibers, and item instances.
//!
//! Item names are not stored in definitions — the [`Id`](crate::primitive::id::Id)
//! doubles as the localization key. Display names are resolved at render time
//! from language-specific localization files.

mod category;
mod data;
mod def;
mod effect;
mod instance;

pub use category::ItemCategory;
pub use data::{ArmorSlot, ItemData, RelicStability};
pub use def::{CaliberDef, ItemDef};
pub use effect::{Effect, EffectTarget};
pub use instance::{Authenticity, Item};
