//! Type-specific item data carried by definitions.

mod ammo;
mod armor;
mod attachment;
mod consumable;
mod document;
mod relic;
mod throwable;
mod weapon;

use serde::{Deserialize, Serialize};

pub use self::ammo::AmmoData;
pub use self::armor::{ArmorData, ArmorSlot};
pub use self::attachment::AttachmentData;
pub use self::consumable::ConsumableData;
pub use self::document::DocumentData;
pub use self::relic::{RelicData, RelicStability};
pub use self::throwable::ThrowableData;
pub use self::weapon::{FireMode, WeaponData};
use super::category::ItemCategory;
use crate::primitive::IdMarker;

/// Marker for caliber IDs (ammo / weapon link).
pub struct Caliber;
impl IdMarker for Caliber {}

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
