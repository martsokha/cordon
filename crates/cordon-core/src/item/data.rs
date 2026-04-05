//! Type-specific item data carried by definitions.

use serde::{Deserialize, Serialize};

use super::category::ItemCategory;
use super::effect::Effect;
use crate::primitive::duration::Duration;
use crate::primitive::id::{Caliber, Id, Item};

/// Which armor slot this piece of armor occupies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArmorSlot {
    /// Body armor: jackets, vests, suits.
    Suit,
    /// Head protection: helmets, balaclavas.
    Helmet,
}

/// Stability state of a relic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelicStability {
    /// Contained properly, safe to store and sell.
    Stable,
    /// Not contained, degrades over time, may harm handler.
    Unstable,
    /// Depleted or damaged, minimal value.
    Inert,
}

/// Weapon fire mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FireMode {
    /// One shot per trigger pull.
    Semi,
    /// Fixed-length burst per trigger pull (e.g., 2-round for AN-94).
    Burst(u8),
    /// Continuous fire while trigger held.
    Auto,
}

/// Data for consumable items (food, medicine, drinks, pills).
///
/// Each effect carries its own duration. A medkit might have an
/// instant heal effect and a 10-second anti-bleeding effect.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsumableData {
    /// Effects applied when consumed. Each has its own duration.
    pub effects: Vec<Effect>,
    /// Seconds to consume this item (animation/use time).
    pub use_time: Duration,
    /// Days until spoilage. `None` means it never spoils.
    pub spoil_days: Option<u32>,
}

/// Data for throwable items (grenades, molotovs, smoke).
///
/// Each effect carries its own duration and optional aoe radius.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThrowableData {
    /// Effects applied on impact. Each has its own duration and aoe.
    pub effects: Vec<Effect>,
    /// Seconds to prime and throw (animation/use time).
    pub use_time: Duration,
}

/// Data for ammunition items.
///
/// Comes in boxes — [`quantity`](AmmoData::quantity) is rounds per box
/// as purchased. References a caliber by ID; weapons that fire the
/// same caliber ID can use this ammo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AmmoData {
    /// Caliber ID this ammo belongs to (e.g., `"9x18mm"`).
    pub caliber: Id<Caliber>,
    /// Base damage per round.
    pub damage: f32,
    /// Armor penetration value (0.0–1.0). Higher = better against armor.
    pub penetration: f32,
    /// Number of rounds per box.
    pub quantity: u32,
}

/// Data for weapon items (firearms).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeaponData {
    /// Caliber ID this weapon fires. Must match an ammo item's caliber.
    pub caliber: Id<Caliber>,
    /// Available fire modes (e.g., `[Semi, Auto]` for an AK-74).
    pub fire_modes: Vec<FireMode>,
    /// Rounds per second at full auto/burst.
    pub fire_rate: f32,
    /// Base accuracy (0.0–1.0). Higher = tighter spread.
    pub accuracy: f32,
    /// Recoil per shot (0.0–1.0). Higher = more spread over sustained fire.
    pub recoil: f32,
    /// Magazine capacity in rounds.
    pub magazine: u32,
    /// Effective range in meters.
    pub effective_range: f32,
    /// Whether this weapon is suppressed (affects runner stealth missions).
    pub suppressed: bool,
}

/// Data for armor items (body armor or head protection).
///
/// Armor occupies its own equipment slot (not inventory slots) and
/// can grant bonus inventory slots while equipped.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmorData {
    /// Which slot this armor occupies.
    pub slot: ArmorSlot,
    /// Ballistic protection (0.0–1.0). Fraction of bullet damage absorbed.
    pub ballistic_protection: f32,
    /// Radiation protection (0.0–1.0). Fraction of radiation absorbed.
    pub radiation_protection: f32,
    /// Extra inventory slots granted while wearing this armor.
    pub bonus_slots: u8,
}

/// Data for Zone relics with anomalous properties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelicData {
    /// Default stability when found. Affects storage requirements.
    pub default_stability: RelicStability,
    /// Passive effects while carried. Applied continuously to the
    /// carrier. If an effect has an [`aoe`](Effect::aoe), it also
    /// affects nearby characters. Duration is ignored.
    pub carried_effects: Vec<Effect>,
}

/// Data for document items (intel, PDAs, reports).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentData {
    /// Whether this document is encrypted and requires decryption
    /// software to read (and sell at full value).
    pub encrypted: bool,
}

/// Data for weapon attachments (underbarrel launchers, scopes, etc.).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttachmentData {
    /// Caliber ID of launched grenades, if this is a launcher.
    pub launcher_caliber: Option<Id<Caliber>>,
    /// Weapon IDs this attachment fits on.
    pub compatible_weapons: Vec<Id<Item>>,
    /// Accuracy modifier when attached (additive, e.g., +0.05).
    pub accuracy_modifier: f32,
    /// Recoil modifier when attached (additive, e.g., -0.1).
    pub recoil_modifier: f32,
}

/// Type-specific data carried by an [`ItemDef`](super::ItemDef).
///
/// Each variant wraps a dedicated data struct. The compiler enforces
/// that a weapon always has a caliber, armor always has a slot, etc.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ItemData {
    /// Food, medicine, drinks, pills.
    Consumable(ConsumableData),
    /// Grenades, molotovs, smoke.
    Throwable(ThrowableData),
    /// Boxes of ammunition.
    Ammo(AmmoData),
    /// Firearms.
    Weapon(WeaponData),
    /// Body armor or head protection.
    Armor(ArmorData),
    /// Zone relics with anomalous properties.
    Relic(RelicData),
    /// Intel: PDAs, reports, patrol routes, classified data.
    Document(DocumentData),
    /// Experimental equipment (scanners, dampeners, jammers).
    Tech,
    /// Weapon attachments (underbarrel launchers, scopes, etc.).
    Attachment(AttachmentData),
}

impl ItemData {
    /// Get the simple [`ItemCategory`] tag for this data variant.
    pub fn category(&self) -> ItemCategory {
        match self {
            ItemData::Consumable(_) => ItemCategory::Consumable,
            ItemData::Throwable(_) => ItemCategory::Throwable,
            ItemData::Ammo(_) => ItemCategory::Ammo,
            ItemData::Weapon(_) => ItemCategory::Weapon,
            ItemData::Armor(_) => ItemCategory::Armor,
            ItemData::Relic(_) => ItemCategory::Relic,
            ItemData::Document(_) => ItemCategory::Document,
            ItemData::Tech => ItemCategory::Tech,
            ItemData::Attachment(_) => ItemCategory::Attachment,
        }
    }
}
