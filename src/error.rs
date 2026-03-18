//! Error types for api2cli.

use thiserror::Error;

/// The primary error type for this crate.
#[derive(Error, Debug)]
pub enum Error {
    /// Failed to load the spec from a URL or file path.
    #[error("Failed to load spec from '{url}': {message}")]
    SpecLoad { url: String, message: String },

    /// The spec is structurally invalid or missing required fields.
    #[error("Invalid OpenAPI spec: {0}")]
    InvalidSpec(String),

    /// The OpenAPI version is not supported (must be 2.0 or 3.x).
    #[error("Unsupported OpenAPI version: {0}")]
    UnsupportedVersion(String),

    /// No matching operation was found for the given subcommand.
    #[error("Operation not found: '{0}'")]
    OperationNotFound(String),

    /// An HTTP request to the API failed.
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// A filesystem I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to serialize/deserialize JSON.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Failed to parse YAML.
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

/// Convenience alias for `Result<T, Error>`.
pub type Result<T> = std::result::Result<T, Error>;
