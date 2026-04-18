//! Intel definitions: data-driven pieces of information the player
//! discovers through events, quests, dialogue, and radio broadcasts.
//!
//! [`IntelDef`] is loaded from JSON config files. Title and description
//! are derived from the ID via localisation keys:
//! `{id}_title` and `{id}_description`.

use serde::{Deserialize, Serialize};

use crate::primitive::{Duration, Id, IdMarker};

/// Marker for intel definition IDs.
pub struct Intel;
impl IdMarker for Intel {}

/// An intel definition loaded from config.
///
/// Title and description are localised: `{id}_title` and
/// `{id}_description`. No text fields here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelDef {
    /// Unique identifier and localisation key root.
    pub id: Id<Intel>,
    /// How long this intel stays relevant after being granted.
    /// `None` means it never expires.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_after: Option<Duration>,
}
