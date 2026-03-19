//! High-level handle to an iTerm2 session (terminal pane).

use crate::connection::Connection;
use crate::error::{Error, Result};
use crate::proto;
use crate::request;
use crate::validate;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};

/// A handle to an iTerm2 session (a single terminal pane).
///
/// Provides methods to send text, read the terminal buffer, split panes,
/// manage variables/properties, and more.
pub struct Session<S> {
    /// The unique session identifier.
    pub id: String,
    /// The session's title, if available.
    pub title: Option<String>,
    conn: Arc<Connection<S>>,
}

impl<S: AsyncRead + AsyncWrite + Unpin + Send + 'static> Session<S> {
    /// Create a session handle. Validates the session ID.
    pub fn new(id: String, title: Option<String>, conn: Arc<Connection<S>>) -> Result<Self> {
        validate::identifier(&id, "session")?;
        Ok(Self { id, title, conn })
    }

    /// Create without validation — used internally when IDs come from the server.
    pub(crate) fn new_unchecked(id: String, title: Option<String>, conn: Arc<Connection<S>>) -> Self {
        Self { id, title, conn }
    }

    /// Send text to the session as if typed on the keyboard.
    pub async fn send_text(&self, text: &str) -> Result<()> {
        validate::text_len(text)?;
        let resp = self.conn.call(request::send_text(&self.id, text)).await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::SendTextResponse(r)) => {
                check_status_i32(r.status, "SendText")
            }
            _ => Err(Error::UnexpectedResponse {
                expected: "SendTextResponse",
            }),
        }
    }

    /// Get the current visible screen contents as lines of text.
    pub async fn get_screen_contents(&self) -> Result<Vec<String>> {
        let resp = self
            .conn
            .call(request::get_buffer_screen(&self.id))
            .await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::GetBufferResponse(r)) => {
                check_buffer_status(r.status)?;
                Ok(r.contents
                    .into_iter()
                    .map(|line| line.text.unwrap_or_default())
                    .collect())
            }
            _ => Err(Error::UnexpectedResponse {
                expected: "GetBufferResponse",
            }),
        }
    }

    /// Get the last N lines from the scrollback buffer.
    pub async fn get_buffer_lines(&self, trailing_lines: i32) -> Result<Vec<String>> {
        let resp = self
            .conn
            .call(request::get_buffer_trailing(&self.id, trailing_lines))
            .await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::GetBufferResponse(r)) => {
                check_buffer_status(r.status)?;
                Ok(r.contents
                    .into_iter()
                    .map(|line| line.text.unwrap_or_default())
                    .collect())
            }
            _ => Err(Error::UnexpectedResponse {
                expected: "GetBufferResponse",
            }),
        }
    }

    /// Split this session's pane. Returns the new session ID(s).
    pub async fn split(
        &self,
        direction: proto::split_pane_request::SplitDirection,
        before: bool,
        profile_name: Option<&str>,
    ) -> Result<Vec<String>> {
        let resp = self
            .conn
            .call(request::split_pane(&self.id, direction, before, profile_name))
            .await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::SplitPaneResponse(r)) => {
                check_split_status(r.status)?;
                Ok(r.session_id)
            }
            _ => Err(Error::UnexpectedResponse {
                expected: "SplitPaneResponse",
            }),
        }
    }

    /// Get a session variable by name. Returns JSON-encoded value.
    pub async fn get_variable(&self, name: &str) -> Result<Option<String>> {
        let resp = self
            .conn
            .call(request::get_variable_session(
                &self.id,
                vec![name.to_string()],
            ))
            .await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::VariableResponse(r)) => {
                check_variable_status(r.status)?;
                Ok(r.values.into_iter().next())
            }
            _ => Err(Error::UnexpectedResponse {
                expected: "VariableResponse",
            }),
        }
    }

    /// Set a session variable. Name must start with `user.`. Value must be valid JSON.
    pub async fn set_variable(&self, name: &str, json_value: &str) -> Result<()> {
        validate::json_value(json_value)?;
        let resp = self
            .conn
            .call(request::set_variable_session(
                &self.id,
                vec![(name.to_string(), json_value.to_string())],
            ))
            .await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::VariableResponse(r)) => {
                check_variable_status(r.status)
            }
            _ => Err(Error::UnexpectedResponse {
                expected: "VariableResponse",
            }),
        }
    }

    /// Get profile properties for this session.
    pub async fn get_profile_property(&self, keys: Vec<String>) -> Result<Vec<proto::ProfileProperty>> {
        let resp = self
            .conn
            .call(request::get_profile_property(&self.id, keys))
            .await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::GetProfilePropertyResponse(r)) => {
                check_status_i32(r.status, "GetProfileProperty")?;
                Ok(r.properties)
            }
            _ => Err(Error::UnexpectedResponse {
                expected: "GetProfilePropertyResponse",
            }),
        }
    }

    /// Set a profile property on this session's copy of the profile. Value must be valid JSON.
    pub async fn set_profile_property(&self, key: &str, json_value: &str) -> Result<()> {
        validate::json_value(json_value)?;
        let resp = self
            .conn
            .call(request::set_profile_property_session(
                &self.id, key, json_value,
            ))
            .await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::SetProfilePropertyResponse(r)) => {
                check_status_i32(r.status, "SetProfileProperty")
            }
            _ => Err(Error::UnexpectedResponse {
                expected: "SetProfilePropertyResponse",
            }),
        }
    }

    /// Inject bytes into the terminal as if produced by the running program.
    pub async fn inject(&self, data: Vec<u8>) -> Result<()> {
        let resp = self
            .conn
            .call(request::inject(vec![self.id.clone()], data))
            .await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::InjectResponse(r)) => {
                for status in &r.status {
                    if *status != proto::inject_response::Status::Ok as i32 {
                        return Err(Error::Status(format!(
                            "Inject failed with status: {status}"
                        )));
                    }
                }
                Ok(())
            }
            _ => Err(Error::UnexpectedResponse {
                expected: "InjectResponse",
            }),
        }
    }

    /// Restart the session's shell process.
    pub async fn restart(&self, only_if_exited: bool) -> Result<()> {
        let resp = self
            .conn
            .call(request::restart_session(&self.id, only_if_exited))
            .await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::RestartSessionResponse(r)) => {
                check_status_i32(r.status, "RestartSession")
            }
            _ => Err(Error::UnexpectedResponse {
                expected: "RestartSessionResponse",
            }),
        }
    }

    /// Close this session. If `force` is true, skip the confirmation prompt.
    pub async fn close(&self, force: bool) -> Result<()> {
        let resp = self
            .conn
            .call(request::close_sessions(vec![self.id.clone()], force))
            .await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::CloseResponse(_r)) => Ok(()),
            _ => Err(Error::UnexpectedResponse {
                expected: "CloseResponse",
            }),
        }
    }

    /// Activate this session (bring its window to front and select it).
    pub async fn activate(&self) -> Result<()> {
        let resp = self
            .conn
            .call(request::activate_session(&self.id))
            .await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::ActivateResponse(r)) => {
                check_status_i32(r.status, "Activate")
            }
            _ => Err(Error::UnexpectedResponse {
                expected: "ActivateResponse",
            }),
        }
    }

    /// Get metadata about the current shell prompt (command, working directory, state).
    pub async fn get_prompt(&self) -> Result<proto::GetPromptResponse> {
        let resp = self.conn.call(request::get_prompt(&self.id)).await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::GetPromptResponse(r)) => Ok(r),
            _ => Err(Error::UnexpectedResponse {
                expected: "GetPromptResponse",
            }),
        }
    }

    /// Get a reference to the underlying connection.
    pub fn connection(&self) -> &Connection<S> {
        &self.conn
    }
}

fn check_status_i32(status: Option<i32>, op: &str) -> Result<()> {
    match status {
        Some(0) | None => Ok(()),
        Some(code) => Err(Error::Status(format!("{op} returned status {code}"))),
    }
}

fn check_buffer_status(status: Option<i32>) -> Result<()> {
    check_status_i32(status, "GetBuffer")
}

fn check_split_status(status: Option<i32>) -> Result<()> {
    check_status_i32(status, "SplitPane")
}

fn check_variable_status(status: Option<i32>) -> Result<()> {
    check_status_i32(status, "Variable")
}
