//! Error types for iTerm2 client operations.

use std::io;

const MAX_SERVER_ERROR_LEN: usize = 512;

/// Errors that can occur when communicating with iTerm2.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// WebSocket protocol error.
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    /// I/O error (socket, file system, etc.).
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Failed to decode a protobuf message from the server.
    #[error("Protobuf decode error: {0}")]
    Decode(#[from] prost::DecodeError),

    /// The server returned an error string in the response.
    #[error("API error from server: {0}")]
    Api(String),

    /// An API operation returned a non-OK status code.
    #[error("Non-OK status: {0}")]
    Status(String),

    /// Authentication failed (bad credentials, osascript failure, etc.).
    #[error("Authentication failed: {0}")]
    Auth(String),

    /// The WebSocket connection was closed unexpectedly.
    #[error("Connection closed")]
    ConnectionClosed,

    /// A request did not receive a response within the timeout period.
    #[error("Request timed out after {0:?}")]
    Timeout(std::time::Duration),

    /// The server returned a response type that doesn't match the request.
    #[error("Unexpected response: expected {expected}, got different submessage")]
    UnexpectedResponse {
        /// The response type that was expected.
        expected: &'static str,
    },
}

/// Create an [`Error::Api`], truncating the server message to prevent
/// unbounded or malicious strings from propagating through error chains.
pub(crate) fn api_error(server_msg: &str) -> Error {
    if server_msg.len() <= MAX_SERVER_ERROR_LEN {
        Error::Api(server_msg.to_string())
    } else {
        let truncated: String = server_msg.chars().take(MAX_SERVER_ERROR_LEN).collect();
        Error::Api(format!("{}... [truncated]", truncated))
    }
}

/// A specialized `Result` type for iTerm2 client operations.
pub type Result<T> = std::result::Result<T, Error>;
