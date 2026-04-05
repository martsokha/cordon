//! Type-specific item data carried by definitions.

use serde::{Deserialize, Serialize};

use crate::primitive::duration::Duration;
use crate::primitive::id::Id;
use super::category::ItemCategory;
use super::effect::Effect;

/// Which armor slot this piece of armor occupies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArmorSlot {
    /// Body armor: jackets, vests, suits.
    Suit,
    /// Head protection: helmets, balaclavas.
    Helmet,
}

/// Stability state of a relic. Part of [`ItemData::Relic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelicStability {
    /// Contained properly, safe to store and sell.
    Stable,
    /// Not contained, degrades over time, may harm handler.
    Unstable,
    /// Depleted or damaged, minimal value.
    Inert,
}

/// Type-specific data carried by an [`ItemDef`](super::ItemDef).
///
/// Each variant holds the data unique to that item category. The
/// compiler enforces that a weapon always has a caliber, armor always
/// has a slot and protection values, etc.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ItemData {
    /// Food, medicine, drinks, pills — anything used on yourself.
    ///
    /// Each effect carries its own duration. A medkit might have an
    /// instant heal effect and a 10-second anti-bleeding effect.
    Consumable {
        /// Effects applied when consumed. Each has its own duration.
        effects: Vec<Effect>,
        /// Seconds to consume this item (animation/use time).
        use_time: Duration,
        /// Days until spoilage. `None` means it never spoils.
        spoil_days: Option<u32>,
    },

    /// Grenades, molotovs, smoke — anything thrown.
    ///
    /// Each effect carries its own duration and optional aoe radius.
    Throwable {
        /// Effects applied on impact. Each has its own duration and aoe.
        effects: Vec<Effect>,
        /// Seconds to prime and throw (animation/use time).
        use_time: Duration,
    },

    /// Boxes of ammunition. References a caliber by [`Id`].
    Ammo {
        /// Caliber ID this ammo belongs to (e.g., `"9x18mm"`).
        caliber: Id,
        /// Base damage per round.
        damage: f32,
        /// Armor penetration value (0.0–1.0). Higher = better against armor.
        penetration: f32,
    },

    /// Firearms.
    Weapon {
        /// Caliber ID this weapon fires.
        caliber: Id,
        /// Rounds per minute.
        fire_rate: f32,
        /// Base accuracy (0.0–1.0). Higher = tighter spread.
        accuracy: f32,
        /// Magazine capacity in rounds.
        magazine: u32,
        /// Effective range in meters.
        effective_range: f32,
    },

    /// Body armor or head protection.
    Armor {
        /// Which slot this armor occupies.
        slot: ArmorSlot,
        /// Ballistic protection (0.0–1.0). Fraction of bullet damage absorbed.
        ballistic_protection: f32,
        /// Radiation protection (0.0–1.0). Fraction of radiation absorbed.
        radiation_protection: f32,
        /// Extra inventory slots granted while wearing this armor.
        /// Vests and rigs add carrying capacity.
        bonus_slots: u32,
    },

    /// Zone relics with anomalous properties.
    Relic {
        /// Default stability when found. Affects storage requirements.
        default_stability: RelicStability,
        /// Passive effects while carried (e.g., minor radiation absorption).
        carried_effects: Vec<Effect>,
    },

    /// Intel: PDAs, reports, patrol routes, classified data.
    Document,

    /// Experimental equipment (scanners, dampeners, jammers).
    Tech,

    /// Weapon attachments (underbarrel launchers, etc.).
    Attachment {
        /// Caliber ID of launched grenades, if this is a launcher.
        launcher_caliber: Option<Id>,
        /// Weapon IDs this attachment fits on.
        compatible_weapons: Vec<Id>,
    },
}

impl ItemData {
    /// Get the simple [`ItemCategory`] tag for this data variant.
    pub fn category(&self) -> ItemCategory {
        match self {
            ItemData::Consumable { .. } => ItemCategory::Consumable,
            ItemData::Throwable { .. } => ItemCategory::Throwable,
            ItemData::Ammo { .. } => ItemCategory::Ammo,
            ItemData::Weapon { .. } => ItemCategory::Weapon,
            ItemData::Armor { .. } => ItemCategory::Armor,
            ItemData::Relic { .. } => ItemCategory::Relic,
            ItemData::Document => ItemCategory::Document,
            ItemData::Tech => ItemCategory::Tech,
            ItemData::Attachment { .. } => ItemCategory::Attachment,
        }
    }
}
