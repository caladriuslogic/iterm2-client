//! High-level handle to an iTerm2 window.

use crate::connection::Connection;
use crate::error::{Error, Result};
use crate::proto;
use crate::request;
use crate::validate;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};

/// A handle to an iTerm2 window.
pub struct Window<S> {
    /// The unique window identifier.
    pub id: String,
    conn: Arc<Connection<S>>,
}

impl<S: AsyncRead + AsyncWrite + Unpin + Send + 'static> Window<S> {
    /// Create a window handle. Validates the window ID.
    pub fn new(id: String, conn: Arc<Connection<S>>) -> Result<Self> {
        validate::identifier(&id, "window")?;
        Ok(Self { id, conn })
    }

    pub(crate) fn new_unchecked(id: String, conn: Arc<Connection<S>>) -> Self {
        Self { id, conn }
    }

    /// Create a new tab in this window, optionally using a named profile.
    pub async fn create_tab(&self, profile_name: Option<&str>) -> Result<CreateTabResult> {
        let resp = self
            .conn
            .call(request::create_tab(profile_name, Some(&self.id)))
            .await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::CreateTabResponse(r)) => {
                check_status_i32(r.status, "CreateTab")?;
                Ok(CreateTabResult {
                    tab_id: r.tab_id.map(|id| id.to_string()),
                    session_id: r.session_id,
                    window_id: r.window_id,
                })
            }
            _ => Err(Error::UnexpectedResponse {
                expected: "CreateTabResponse",
            }),
        }
    }

    /// Activate this window (bring it to the front).
    pub async fn activate(&self) -> Result<()> {
        let resp = self.conn.call(request::activate_window(&self.id)).await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::ActivateResponse(r)) => {
                check_status_i32(r.status, "Activate")
            }
            _ => Err(Error::UnexpectedResponse {
                expected: "ActivateResponse",
            }),
        }
    }

    /// Close this window. If `force` is true, skip the confirmation prompt.
    pub async fn close(&self, force: bool) -> Result<()> {
        let resp = self
            .conn
            .call(request::close_windows(vec![self.id.clone()], force))
            .await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::CloseResponse(_)) => Ok(()),
            _ => Err(Error::UnexpectedResponse {
                expected: "CloseResponse",
            }),
        }
    }

    /// Get a window property (e.g. `"frame"`, `"fullscreen"`). Returns JSON-encoded value.
    pub async fn get_property(&self, name: &str) -> Result<Option<String>> {
        let resp = self
            .conn
            .call(request::get_property_window(&self.id, name))
            .await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::GetPropertyResponse(r)) => {
                check_status_i32(r.status, "GetProperty")?;
                Ok(r.json_value)
            }
            _ => Err(Error::UnexpectedResponse {
                expected: "GetPropertyResponse",
            }),
        }
    }

    /// Set a window property. Value must be valid JSON.
    pub async fn set_property(&self, name: &str, json_value: &str) -> Result<()> {
        validate::json_value(json_value)?;
        let resp = self
            .conn
            .call(request::set_property_window(&self.id, name, json_value))
            .await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::SetPropertyResponse(r)) => {
                check_status_i32(r.status, "SetProperty")
            }
            _ => Err(Error::UnexpectedResponse {
                expected: "SetPropertyResponse",
            }),
        }
    }

    /// Get a window variable by name. Returns JSON-encoded value.
    pub async fn get_variable(&self, name: &str) -> Result<Option<String>> {
        let resp = self
            .conn
            .call(request::get_variable_window(
                &self.id,
                vec![name.to_string()],
            ))
            .await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::VariableResponse(r)) => {
                check_status_i32(r.status, "Variable")?;
                Ok(r.values.into_iter().next())
            }
            _ => Err(Error::UnexpectedResponse {
                expected: "VariableResponse",
            }),
        }
    }

    /// Get a reference to the underlying connection.
    pub fn connection(&self) -> &Connection<S> {
        &self.conn
    }
}

/// Result of [`Window::create_tab`].
pub struct CreateTabResult {
    pub tab_id: Option<String>,
    pub session_id: Option<String>,
    pub window_id: Option<String>,
}

fn check_status_i32(status: Option<i32>, op: &str) -> Result<()> {
    match status {
        Some(0) | None => Ok(()),
        Some(code) => Err(Error::Status(format!("{op} returned status {code}"))),
    }
}
