//! Attachment item data (underbarrel launchers, scopes, etc.).

use serde::{Deserialize, Serialize};

use super::Caliber;
use crate::item::def::Item;
use crate::primitive::Id;

/// Data for weapon attachments.
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
