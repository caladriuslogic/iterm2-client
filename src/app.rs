use crate::connection::Connection;
use crate::error::{Error, Result};
use crate::proto;
use crate::request;
use crate::session::Session;
use crate::tab::Tab;
use crate::window::Window;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};

pub struct App<S> {
    conn: Arc<Connection<S>>,
}

impl<S: AsyncRead + AsyncWrite + Unpin + Send + 'static> App<S> {
    pub fn new(conn: Connection<S>) -> Self {
        Self {
            conn: Arc::new(conn),
        }
    }

    pub fn from_arc(conn: Arc<Connection<S>>) -> Self {
        Self { conn }
    }

    pub async fn list_sessions(&self) -> Result<ListSessionsResult<S>> {
        let resp = self.conn.call(request::list_sessions()).await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::ListSessionsResponse(r)) => {
                Ok(self.parse_list_sessions(r))
            }
            _ => Err(Error::UnexpectedResponse {
                expected: "ListSessionsResponse",
            }),
        }
    }

    fn parse_list_sessions(&self, resp: proto::ListSessionsResponse) -> ListSessionsResult<S> {
        let mut windows = Vec::new();
        for w in resp.windows {
            let window_id = w.window_id.unwrap_or_default();
            let mut tabs = Vec::new();
            for t in w.tabs {
                let tab_id = t.tab_id.unwrap_or_default();
                let sessions = collect_sessions_from_tree(t.root.as_ref(), &self.conn);
                tabs.push(TabInfo {
                    tab: Tab::new_unchecked(tab_id, Arc::clone(&self.conn)),
                    sessions,
                });
            }
            windows.push(WindowInfo {
                window: Window::new_unchecked(window_id, Arc::clone(&self.conn)),
                tabs,
            });
        }

        let buried_sessions = resp
            .buried_sessions
            .into_iter()
            .map(|s| {
                Session::new_unchecked(
                    s.unique_identifier.unwrap_or_default(),
                    s.title,
                    Arc::clone(&self.conn),
                )
            })
            .collect();

        ListSessionsResult {
            windows,
            buried_sessions,
        }
    }

    pub async fn create_tab(
        &self,
        profile_name: Option<&str>,
        window_id: Option<&str>,
    ) -> Result<CreateTabResult<S>> {
        let resp = self
            .conn
            .call(request::create_tab(profile_name, window_id))
            .await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::CreateTabResponse(r)) => {
                check_status_i32(r.status, "CreateTab")?;
                let session_id = r.session_id.unwrap_or_default();
                let tab_id = r.tab_id.map(|id| id.to_string()).unwrap_or_default();
                let window_id = r.window_id.unwrap_or_default();
                Ok(CreateTabResult {
                    window: Window::new_unchecked(window_id, Arc::clone(&self.conn)),
                    tab: Tab::new_unchecked(tab_id, Arc::clone(&self.conn)),
                    session: Session::new_unchecked(session_id, None, Arc::clone(&self.conn)),
                })
            }
            _ => Err(Error::UnexpectedResponse {
                expected: "CreateTabResponse",
            }),
        }
    }

    pub async fn focus(&self) -> Result<Vec<proto::FocusChangedNotification>> {
        let resp = self.conn.call(request::focus()).await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::FocusResponse(r)) => {
                Ok(r.notifications)
            }
            _ => Err(Error::UnexpectedResponse {
                expected: "FocusResponse",
            }),
        }
    }

    pub async fn activate(&self, raise_all: bool, ignoring_other_apps: bool) -> Result<()> {
        let resp = self
            .conn
            .call(request::activate_app(raise_all, ignoring_other_apps))
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

    pub async fn list_profiles(
        &self,
        properties: Vec<String>,
        guids: Vec<String>,
    ) -> Result<proto::ListProfilesResponse> {
        let resp = self
            .conn
            .call(request::list_profiles(properties, guids))
            .await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::ListProfilesResponse(r)) => Ok(r),
            _ => Err(Error::UnexpectedResponse {
                expected: "ListProfilesResponse",
            }),
        }
    }

    pub async fn begin_transaction(&self) -> Result<()> {
        let resp = self.conn.call(request::begin_transaction()).await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::TransactionResponse(r)) => {
                check_status_i32(r.status, "Transaction")
            }
            _ => Err(Error::UnexpectedResponse {
                expected: "TransactionResponse",
            }),
        }
    }

    pub async fn end_transaction(&self) -> Result<()> {
        let resp = self.conn.call(request::end_transaction()).await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::TransactionResponse(r)) => {
                check_status_i32(r.status, "Transaction")
            }
            _ => Err(Error::UnexpectedResponse {
                expected: "TransactionResponse",
            }),
        }
    }

    pub async fn list_color_presets(&self) -> Result<Vec<String>> {
        let resp = self.conn.call(request::list_color_presets()).await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::ColorPresetResponse(r)) => {
                check_status_i32(r.status, "ColorPreset")?;
                match r.response {
                    Some(proto::color_preset_response::Response::ListPresets(lp)) => Ok(lp.name),
                    _ => Ok(vec![]),
                }
            }
            _ => Err(Error::UnexpectedResponse {
                expected: "ColorPresetResponse",
            }),
        }
    }

    pub async fn list_arrangements(&self) -> Result<Vec<String>> {
        let resp = self.conn.call(request::list_arrangements()).await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::SavedArrangementResponse(r)) => {
                check_status_i32(r.status, "SavedArrangement")?;
                Ok(r.names)
            }
            _ => Err(Error::UnexpectedResponse {
                expected: "SavedArrangementResponse",
            }),
        }
    }

    pub async fn get_broadcast_domains(&self) -> Result<Vec<proto::BroadcastDomain>> {
        let resp = self.conn.call(request::get_broadcast_domains()).await?;
        match resp.submessage {
            Some(proto::server_originated_message::Submessage::GetBroadcastDomainsResponse(r)) => {
                Ok(r.broadcast_domains)
            }
            _ => Err(Error::UnexpectedResponse {
                expected: "GetBroadcastDomainsResponse",
            }),
        }
    }

    pub fn subscribe_notifications(&self) -> tokio::sync::broadcast::Receiver<proto::Notification> {
        self.conn.subscribe_notifications()
    }

    pub fn connection(&self) -> &Connection<S> {
        &self.conn
    }

    pub fn connection_arc(&self) -> Arc<Connection<S>> {
        Arc::clone(&self.conn)
    }
}

fn collect_sessions_from_tree<S: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
    node: Option<&proto::SplitTreeNode>,
    conn: &Arc<Connection<S>>,
) -> Vec<Session<S>> {
    let mut sessions = Vec::new();
    if let Some(node) = node {
        for link in &node.links {
            if let Some(child) = &link.child {
                match child {
                    proto::split_tree_node::split_tree_link::Child::Session(s) => {
                        sessions.push(Session::new_unchecked(
                            s.unique_identifier.clone().unwrap_or_default(),
                            s.title.clone(),
                            Arc::clone(conn),
                        ));
                    }
                    proto::split_tree_node::split_tree_link::Child::Node(n) => {
                        sessions.extend(collect_sessions_from_tree(Some(n), conn));
                    }
                }
            }
        }
    }
    sessions
}

pub struct ListSessionsResult<S> {
    pub windows: Vec<WindowInfo<S>>,
    pub buried_sessions: Vec<Session<S>>,
}

pub struct WindowInfo<S> {
    pub window: Window<S>,
    pub tabs: Vec<TabInfo<S>>,
}

pub struct TabInfo<S> {
    pub tab: Tab<S>,
    pub sessions: Vec<Session<S>>,
}

pub struct CreateTabResult<S> {
    pub window: Window<S>,
    pub tab: Tab<S>,
    pub session: Session<S>,
}

fn check_status_i32(status: Option<i32>, op: &str) -> Result<()> {
    match status {
        Some(0) | None => Ok(()),
        Some(code) => Err(Error::Status(format!("{op} returned status {code}"))),
    }
}
