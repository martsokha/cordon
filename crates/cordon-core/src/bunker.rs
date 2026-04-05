use serde::{Deserialize, Serialize};

use crate::item::ItemStack;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BunkerState {
    // Chain upgrades (current level, 0 = not built)
    pub laptop_level: u8,
    pub radio_level: u8,
    pub storage_level: u8,
    pub counter_level: u8,

    // One-off upgrades
    pub upgrades: Vec<OneOffUpgrade>,

    // Inventory
    pub storage: Vec<ItemStack>,
    pub hidden_storage: Vec<ItemStack>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OneOffUpgrade {
    // Inspection
    MagnifyingLens,
    RelicScanner,
    AdvancedToolkit,
    GeigerCounter,

    // Stabilization
    RelicStabilizer,

    // Storage
    Fridge,
    Freezer,
    RelicContainmentUnit,
    SecretCompartment,
    ClimateControl,

    // Security
    ReinforcedDoor,
    AlarmSystem,
    Barricades,

    // Intel
    ZoneMap,
    MapSubscription,
    MapBoard,
    ThreatTracker,
    DeepScanner,
    IntelNetwork,
    DecryptionSoftware,
    FactionDossiers,

    // Quality of life
    Generator,
    BackupGenerator,
    Cot,
    Lockbox,
    Ledger,
    Scale,
}

impl BunkerState {
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

    pub fn has_upgrade(&self, upgrade: OneOffUpgrade) -> bool {
        self.upgrades.contains(&upgrade)
    }

    pub fn has_power(&self) -> bool {
        self.has_upgrade(OneOffUpgrade::Generator)
    }

    pub fn storage_capacity(&self) -> u32 {
        match self.storage_level {
            1 => 20,
            2 => 40,
            3 => 80,
            _ => 20,
        }
    }

    pub fn hidden_capacity(&self) -> u32 {
        if self.has_upgrade(OneOffUpgrade::SecretCompartment) {
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
