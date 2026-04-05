//! Error types for data loading.

use std::path::PathBuf;

/// Errors that can occur during asset loading.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to read a file from disk.
    #[error("failed to read {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    /// Failed to parse JSON.
    #[error("failed to parse {path}: {source}")]
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },

    /// A required directory is missing.
    #[error("missing directory: {0}")]
    MissingDir(PathBuf),
}

/// Result type for data loading operations.
pub type Result<T> = std::result::Result<T, Error>;
