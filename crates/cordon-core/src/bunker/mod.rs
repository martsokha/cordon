//! Bunker state, upgrade definitions, and storage.
//!
//! The bunker is the player's base of operations. It has chain upgrades
//! (laptop, radio, storage, counter) and one-off upgrades loaded from config.

use serde::{Deserialize, Serialize};

use crate::economy::item::ItemStack;
use crate::object::id::Id;

/// An upgrade definition loaded from config.
///
/// Upgrades can be one-offs or prerequisites for other upgrades.
/// Some require specific faction standings or chain levels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeDef {
    /// Unique identifier (e.g., `"fridge"`, `"alarm_system"`).
    pub id: Id,
    /// Display name.
    pub name: String,
    /// What this upgrade does.
    pub description: String,
    /// Credit cost to purchase.
    pub cost: u32,
    /// IDs of other upgrades that must be purchased first.
    pub requires: Vec<Id>,
    /// Required chain level: `(chain_name, min_level)`.
    pub requires_chain: Option<(String, u8)>,
    /// Required faction standing: `(faction_id, min_standing)`.
    pub requires_standing: Option<(Id, i8)>,
}

/// The bunker's current state.
///
/// Tracks chain upgrade levels, installed one-off upgrades,
/// and the contents of regular and hidden storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BunkerState {
    /// Laptop chain level (1–5). Affects intel and market visibility.
    pub laptop_level: u8,
    /// Radio chain level (1–5). Determines how far runners can be sent.
    pub radio_level: u8,
    /// Storage chain level (1–3). Determines base storage capacity.
    pub storage_level: u8,
    /// Counter chain level (1–3). Affects NPC trust and inspection tools.
    pub counter_level: u8,

    /// Installed one-off upgrade IDs.
    pub upgrades: Vec<Id>,

    /// Main storage contents.
    pub storage: Vec<ItemStack>,
    /// Hidden storage contents (survives raids, invisible during inspections).
    pub hidden_storage: Vec<ItemStack>,
}

impl BunkerState {
    /// Create a new bunker with all chains at level 1 and no upgrades.
    pub fn new() -> Self {
        Self {
            laptop_level: 1,
            radio_level: 1,
            storage_level: 1,
            counter_level: 1,
            upgrades: Vec::new(),
            storage: Vec::new(),
            hidden_storage: Vec::new(),
        }
    }

    /// Check if a one-off upgrade is installed.
    pub fn has_upgrade(&self, upgrade_id: &Id) -> bool {
        self.upgrades.iter().any(|u| u == upgrade_id)
    }

    /// Whether the bunker has a generator (prevents power outages).
    pub fn has_power(&self) -> bool {
        self.has_upgrade(&Id::new("generator"))
    }

    /// Maximum number of item stacks in main storage.
    pub fn storage_capacity(&self) -> u32 {
        match self.storage_level {
            1 => 20,
            2 => 40,
            3 => 80,
            _ => 20,
        }
    }

    /// Maximum number of item stacks in hidden storage.
    ///
    /// Returns 0 if the secret compartment upgrade is not installed.
    pub fn hidden_capacity(&self) -> u32 {
        if self.has_upgrade(&Id::new("secret_compartment")) {
            10
        } else {
            0
        }
    }
}

impl Default for BunkerState {
    fn default() -> Self {
        Self::new()
    }
}
