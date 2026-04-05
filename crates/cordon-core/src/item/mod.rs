//! Item definitions, effects, inventory, and item instances.
//!
//! Item names are not stored in definitions — the [`Id`](crate::primitive::id::Id)
//! doubles as the localization key. Display names are resolved at render time
//! from language-specific localization files. Calibers are implicit: they exist
//! because ammo and weapon items reference the same caliber ID string.

mod category;
mod data;
mod def;
mod effect;
mod instance;
mod inventory;

pub use category::ItemCategory;
pub use data::{
    AmmoData, ArmorData, ArmorSlot, AttachmentData, ConsumableData, DocumentData, FireMode,
    ItemData, RelicData, RelicStability, ThrowableData, WeaponData,
};
pub use def::{ItemDef, Supplier};
pub use effect::{Effect, EffectTarget};
pub use instance::{Authenticity, Item};
pub use inventory::Inventory;
