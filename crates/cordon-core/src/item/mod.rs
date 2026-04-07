//! Item definitions, effects, inventory, and item instances.
//!
//! Item names are not stored in definitions — the [`Id`](crate::primitive::Id)
//! doubles as the localization key. Display names are resolved at render time
//! from language-specific localization files. Calibers are implicit: they exist
//! because ammo and weapon items reference the same caliber ID string.

mod category;
mod data;
mod def;
mod effect;
mod instance;
mod loadout;
mod stash;

pub use self::category::ItemCategory;
pub use self::data::{
    AmmoData, ArmorData, ArmorSlot, AttachmentData, Caliber, ConsumableData, DocumentData,
    FireMode, ItemData, RelicData, ThrowableData, WeaponData,
};
pub use self::def::{Item, ItemDef, Supplier};
pub use self::effect::{
    EffectDuration, EffectTrigger, PassiveModifier, ResourceTarget, StatTarget, TimedEffect,
    TriggeredEffect,
};
pub use self::instance::{Authenticity, ItemInstance};
pub use self::loadout::{BASE_GENERAL_SLOTS, Loadout, MAX_RELIC_SLOTS};
pub use self::stash::Stash;
