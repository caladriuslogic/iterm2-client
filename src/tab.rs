use crate::connection::Connection;
use crate::error::{Error, Result};
use crate::proto;
use crate::request;
use crate::validate;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};

pub struct Tab<S> {
    pub id: String,
    conn: Arc<Connection<S>>,
}

impl<S: AsyncRead + AsyncWrite + Unpin + Send + 'static> Tab<S> {
    pub fn new(id: String, conn: Arc<Connection<S>>) -> Result<Self> {
        validate::identifier(&id, "tab")?;
        Ok(Self { id, conn })
    }

    pub(crate) fn new_unchecked(id: String, conn: Arc<Connection<S>>) -> Self {
        Self { id, conn }
    }

    pub async fn activate(&self) -> Result<()> {
        let resp = self.conn.call(request::activate_tab(&self.id)).await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::ActivateResponse(r)) => {
                check_status_i32(r.status, "Activate")
            }
            _ => Err(Error::UnexpectedResponse {
                expected: "ActivateResponse",
            }),
        }
    }

    pub async fn close(&self, force: bool) -> Result<()> {
        let resp = self
            .conn
            .call(request::close_tabs(vec![self.id.clone()], force))
            .await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::CloseResponse(_)) => Ok(()),
            _ => Err(Error::UnexpectedResponse {
                expected: "CloseResponse",
            }),
        }
    }

    pub async fn get_variable(&self, name: &str) -> Result<Option<String>> {
        let resp = self
            .conn
            .call(request::get_variable_tab(&self.id, vec![name.to_string()]))
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
