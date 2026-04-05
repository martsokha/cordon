//! Type-specific item data carried by definitions.

use serde::{Deserialize, Serialize};

use super::category::ItemCategory;
use super::effect::Effect;
use crate::primitive::duration::Duration;
use crate::primitive::id::{Id, Caliber, Item};

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

    /// Ammunition. Comes in boxes — [`quantity`](ItemData::Ammo::quantity)
    /// is rounds per box as purchased. References a caliber by ID;
    /// weapons that fire the same caliber ID can use this ammo.
    Ammo {
        /// Caliber ID this ammo belongs to (e.g., `"9x18mm"`).
        caliber: Id<Caliber>,
        /// Base damage per round.
        damage: f32,
        /// Armor penetration value (0.0–1.0). Higher = better against armor.
        penetration: f32,
        /// Number of rounds per box.
        quantity: u32,
    },

    /// Firearms.
    Weapon {
        /// Caliber ID this weapon fires. Must match an ammo item's caliber.
        caliber: Id<Caliber>,
        /// Available fire modes (e.g., `[Semi, Auto]` for an AK-74).
        fire_modes: Vec<FireMode>,
        /// Rounds per minute at full auto/burst.
        fire_rate: f32,
        /// Base accuracy (0.0–1.0). Higher = tighter spread.
        accuracy: f32,
        /// Recoil per shot (0.0–1.0). Higher = more spread over sustained fire.
        recoil: f32,
        /// Magazine capacity in rounds.
        magazine: u32,
        /// Effective range in meters.
        effective_range: f32,
        /// Whether this weapon is suppressed (affects runner stealth missions).
        suppressed: bool,
    },

    /// Body armor or head protection.
    ///
    /// Armor occupies its own equipment slot (not inventory slots) and
    /// can grant bonus inventory slots. The net inventory gain from
    /// wearing armor is [`bonus_slots`](ItemData::Armor::bonus_slots)
    /// (the armor itself doesn't consume inventory slots while equipped).
    Armor {
        /// Which slot this armor occupies.
        slot: ArmorSlot,
        /// Ballistic protection (0.0–1.0). Fraction of bullet damage absorbed.
        ballistic_protection: f32,
        /// Radiation protection (0.0–1.0). Fraction of radiation absorbed.
        radiation_protection: f32,
        /// Extra inventory slots granted while wearing this armor.
        /// Vests and rigs add carrying capacity.
        bonus_slots: u8,
    },

    /// Zone relics with anomalous properties.
    Relic {
        /// Default stability when found. Affects storage requirements.
        default_stability: RelicStability,
        /// Passive effects while carried. Applied continuously to the
        /// carrier. If an effect has an [`aoe`](Effect::aoe), it also
        /// affects nearby characters (e.g., a relic that heals allies
        /// or irradiates everyone within range). Duration is ignored.
        carried_effects: Vec<Effect>,
    },

    /// Intel: PDAs, reports, patrol routes, classified data.
    Document {
        /// Whether this document is encrypted and requires decryption
        /// software to read (and sell at full value).
        encrypted: bool,
    },

    /// Experimental equipment (scanners, dampeners, jammers).
    Tech,

    /// Weapon attachments (underbarrel launchers, scopes, etc.).
    Attachment {
        /// Caliber ID of launched grenades, if this is a launcher.
        launcher_caliber: Option<Id<Caliber>>,
        /// Weapon IDs this attachment fits on.
        compatible_weapons: Vec<Id<Item>>,
        /// Accuracy modifier when attached (additive, e.g., +0.05).
        accuracy_modifier: f32,
        /// Recoil modifier when attached (additive, e.g., -0.1).
        recoil_modifier: f32,
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
            ItemData::Document { .. } => ItemCategory::Document,
            ItemData::Tech => ItemCategory::Tech,
            ItemData::Attachment { .. } => ItemCategory::Attachment,
        }
    }
}
