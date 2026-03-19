//! Rust client for the iTerm2 scripting API.
//!
//! Communicates over WebSocket + Protobuf to control iTerm2 windows, tabs, and
//! sessions programmatically. Covers all 34 operations in the iTerm2 API.
//!
//! # Quick start
//!
//! ```no_run
//! use iterm2_client::{App, Connection};
//!
//! #[tokio::main]
//! async fn main() -> iterm2_client::Result<()> {
//!     let conn = Connection::connect("my-app").await?;
//!     let app = App::new(conn);
//!
//!     let result = app.list_sessions().await?;
//!     let session = &result.windows[0].tabs[0].sessions[0];
//!     session.send_text("echo hello\n").await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Architecture
//!
//! **Low-level**: [`Connection::call()`] sends raw protobuf messages. Use with
//! the [`request`] module builders for full control.
//!
//! **High-level**: [`App`], [`Window`], [`Tab`], and [`Session`] wrap
//! `Arc<Connection>` and provide ergonomic methods with status checking.

/// Generated protobuf types from iTerm2's `api.proto`.
pub mod proto;
/// Error types for all operations.
pub mod error;
/// Authentication: credential resolution from env vars or osascript.
pub mod auth;
/// WebSocket transport (TCP and Unix socket).
pub mod transport;
/// Core connection: request-response matching and notification dispatch.
pub mod connection;
/// Request builders for all 34 iTerm2 API operations.
pub mod request;
/// Notification streams with typed filtering.
pub mod notification;
/// Input validation helpers.
pub mod validate;
/// High-level session handle.
pub mod session;
/// High-level tab handle.
pub mod tab;
/// High-level window handle.
pub mod window;
/// High-level application entry point.
pub mod app;

pub use app::App;
pub use connection::Connection;
pub use error::{Error, Result};
pub use session::Session;
pub use tab::Tab;
pub use window::Window;
