//! Error types for the Asana MCP server.

use thiserror::Error;

/// Errors that can occur when using the Asana client.
#[derive(Debug, Error)]
pub enum Error {
    /// The `ASANA_TOKEN` environment variable is not set or is empty.
    #[error("ASANA_TOKEN environment variable is not set")]
    MissingToken,

    /// The provided token contains invalid characters.
    #[error("invalid token format")]
    InvalidToken,

    /// An HTTP request failed.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Failed to parse a response from the API.
    #[error("failed to parse response: {0}")]
    Parse(#[from] serde_json::Error),

    /// The API returned an error response.
    #[error("API error: {message}")]
    Api {
        /// The error message from the API.
        message: String,
    },

    /// A resource was not found.
    #[error("resource not found: {0}")]
    NotFound(String),
}
