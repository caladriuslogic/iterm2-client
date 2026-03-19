use std::io;

const MAX_SERVER_ERROR_LEN: usize = 512;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Protobuf decode error: {0}")]
    Decode(#[from] prost::DecodeError),

    #[error("API error from server: {0}")]
    Api(String),

    #[error("Non-OK status: {0}")]
    Status(String),

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Request timed out after {0:?}")]
    Timeout(std::time::Duration),

    #[error("Unexpected response: expected {expected}, got different submessage")]
    UnexpectedResponse { expected: &'static str },
}

/// Create an Api error, truncating the server message to prevent
/// unbounded or malicious strings from propagating through error chains.
pub(crate) fn api_error(server_msg: &str) -> Error {
    if server_msg.len() <= MAX_SERVER_ERROR_LEN {
        Error::Api(server_msg.to_string())
    } else {
        let truncated: String = server_msg.chars().take(MAX_SERVER_ERROR_LEN).collect();
        Error::Api(format!("{}... [truncated]", truncated))
    }
}

pub type Result<T> = std::result::Result<T, Error>;
