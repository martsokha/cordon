//! Simple item category tag for filtering and event references.

use serde::{Deserialize, Serialize};

/// Simple item category tag without associated data.
///
/// Used where only the broad category matters: event shortages,
/// targeted search missions, UI filtering, inventory grouping.
/// Does not carry type-specific data — see [`ItemData`](super::ItemData)
/// for that.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemCategory {
    /// Food, medicine, drinks, pills.
    Consumable,
    /// Grenades, molotovs, smoke.
    Throwable,
    /// Boxes of ammunition.
    Ammo,
    /// Firearms.
    Weapon,
    /// Body armor or head protection.
    Armor,
    /// Zone relics.
    Relic,
    /// Intel and documents.
    Document,
    /// Experimental equipment.
    Tech,
    /// Weapon attachments.
    Attachment,
}
