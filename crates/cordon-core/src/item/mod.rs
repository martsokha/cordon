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
mod query;
mod scope;
mod stash;

pub use self::category::ItemCategory;
pub use self::data::{
    AmmoData, ArmorData, ArmorSlot, AttachmentData, Caliber, ConsumableData, DocumentData,
    FireMode, ItemData, RelicData, ThrowableData, WeaponData,
};
pub use self::def::{Item, ItemDef, Supplier};
pub use self::effect::{
    CORRUPTION_HIGH_THRESHOLD, CORRUPTION_LOW_THRESHOLD, EffectTrigger, HP_HIGH_THRESHOLD,
    HP_LOW_THRESHOLD, PERIODIC_INTERVAL_MINUTES, PassiveModifier, ResourceTarget,
    STAMINA_HIGH_THRESHOLD, STAMINA_LOW_THRESHOLD, StatTarget, TimedEffect, TriggeredEffect,
};
pub use self::instance::ItemInstance;
pub use self::loadout::{BASE_GENERAL_SLOTS, Loadout, MAX_RELIC_SLOTS};
pub use self::query::ItemQuery;
pub use self::scope::StashScope;
pub use self::stash::Stash;
