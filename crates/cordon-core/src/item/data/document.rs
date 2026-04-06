//! Document item data (intel, PDAs, reports).

use serde::{Deserialize, Serialize};

/// Data for document items.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentData {
    /// Whether this document is encrypted and requires decryption
    /// software to read (and sell at full value).
    pub encrypted: bool,
}
