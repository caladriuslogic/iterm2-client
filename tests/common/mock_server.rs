#![allow(dead_code)]

use futures_util::{SinkExt, StreamExt};
use iterm2_client::proto;
use prost::Message;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Notify;
use tokio_tungstenite::tungstenite;

pub type Handler = Arc<
    dyn Fn(proto::ClientOriginatedMessage) -> Option<proto::ServerOriginatedMessage>
        + Send
        + Sync,
>;

pub struct MockServer {
    pub addr: std::net::SocketAddr,
    shutdown: Arc<Notify>,
    handle: tokio::task::JoinHandle<()>,
}

impl MockServer {
    pub async fn start(handler: Handler) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let shutdown = Arc::new(Notify::new());
        let shutdown_clone = Arc::clone(&shutdown);

        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    accept = listener.accept() => {
                        if let Ok((stream, _)) = accept {
                            let handler = Arc::clone(&handler);
                            tokio::spawn(handle_connection(stream, handler));
                        }
                    }
                    _ = shutdown_clone.notified() => break,
                }
            }
        });

        MockServer {
            addr,
            shutdown,
            handle,
        }
    }

    pub async fn shutdown(self) {
        self.shutdown.notify_one();
        let _ = self.handle.await;
    }

    pub fn ws_url(&self) -> String {
        format!("ws://{}", self.addr)
    }
}

async fn handle_connection(stream: tokio::net::TcpStream, handler: Handler) {
    let ws_stream = tokio_tungstenite::accept_async(stream).await.unwrap();
    let (mut sink, mut source) = ws_stream.split();

    while let Some(msg) = source.next().await {
        let msg = match msg {
            Ok(tungstenite::Message::Binary(data)) => data,
            Ok(tungstenite::Message::Close(_)) => break,
            Ok(_) => continue,
            Err(_) => break,
        };

        let request = match proto::ClientOriginatedMessage::decode(msg.as_ref()) {
            Ok(r) => r,
            Err(_) => continue,
        };

        let request_id = request.id;

        if let Some(mut response) = handler(request) {
            response.id = request_id;
            let mut buf = Vec::new();
            response.encode(&mut buf).unwrap();
            if sink
                .send(tungstenite::Message::Binary(buf.into()))
                .await
                .is_err()
            {
                break;
            }
        }
    }
}

/// Connect to the mock server, returning a Connection
pub async fn connect_to_mock(
    addr: std::net::SocketAddr,
) -> iterm2_client::Connection<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>> {
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;

    let url = format!("ws://{}", addr);
    let request = url.into_client_request().unwrap();
    let (ws_stream, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    let (sink, source) = ws_stream.split();
    iterm2_client::Connection::from_split(sink, source)
}

/// Create a handler that always returns a ListSessionsResponse
pub fn list_sessions_handler() -> Handler {
    Arc::new(|req| {
        if let Some(proto::client_originated_message::Submessage::ListSessionsRequest(_)) =
            req.submessage
        {
            Some(proto::ServerOriginatedMessage {
                id: req.id,
                submessage: Some(
                    proto::server_originated_message::Submessage::ListSessionsResponse(
                        proto::ListSessionsResponse {
                            windows: vec![proto::list_sessions_response::Window {
                                tabs: vec![proto::list_sessions_response::Tab {
                                    root: Some(proto::SplitTreeNode {
                                        vertical: Some(false),
                                        links: vec![proto::split_tree_node::SplitTreeLink {
                                            child: Some(
                                                proto::split_tree_node::split_tree_link::Child::Session(
                                                    proto::SessionSummary {
                                                        unique_identifier: Some(
                                                            "session-1".to_string(),
                                                        ),
                                                        title: Some("bash".to_string()),
                                                        frame: None,
                                                        grid_size: None,
                                                    },
                                                ),
                                            ),
                                        }],
                                    }),
                                    tab_id: Some("tab-1".to_string()),
                                    tmux_window_id: None,
                                    tmux_connection_id: None,
                                    minimized_sessions: vec![],
                                }],
                                window_id: Some("window-1".to_string()),
                                frame: None,
                                number: Some(0),
                            }],
                            buried_sessions: vec![],
                        },
                    ),
                ),
            })
        } else {
            None
        }
    })
}

/// Create a handler that echoes back OK for common request types
pub fn echo_ok_handler() -> Handler {
    Arc::new(|req| {
        let submessage = match &req.submessage {
            Some(proto::client_originated_message::Submessage::SendTextRequest(_)) => {
                Some(proto::server_originated_message::Submessage::SendTextResponse(
                    proto::SendTextResponse {
                        status: Some(proto::send_text_response::Status::Ok as i32),
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::ListSessionsRequest(_)) => {
                Some(proto::server_originated_message::Submessage::ListSessionsResponse(
                    proto::ListSessionsResponse {
                        windows: vec![],
                        buried_sessions: vec![],
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::GetBufferRequest(_)) => {
                Some(proto::server_originated_message::Submessage::GetBufferResponse(
                    proto::GetBufferResponse {
                        status: Some(proto::get_buffer_response::Status::Ok as i32),
                        #[allow(deprecated)]
                        range: None,
                        contents: vec![
                            proto::LineContents {
                                text: Some("hello world".to_string()),
                                code_points_per_cell: vec![],
                                continuation: None,
                                style: vec![],
                            },
                        ],
                        cursor: None,
                        #[allow(deprecated)]
                        num_lines_above_screen: None,
                        windowed_coord_range: None,
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::CreateTabRequest(_)) => {
                Some(proto::server_originated_message::Submessage::CreateTabResponse(
                    proto::CreateTabResponse {
                        status: Some(proto::create_tab_response::Status::Ok as i32),
                        window_id: Some("window-1".to_string()),
                        tab_id: Some(42),
                        session_id: Some("new-session-1".to_string()),
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::SplitPaneRequest(_)) => {
                Some(proto::server_originated_message::Submessage::SplitPaneResponse(
                    proto::SplitPaneResponse {
                        status: Some(proto::split_pane_response::Status::Ok as i32),
                        session_id: vec!["split-session-1".to_string()],
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::ActivateRequest(_)) => {
                Some(proto::server_originated_message::Submessage::ActivateResponse(
                    proto::ActivateResponse {
                        status: Some(proto::activate_response::Status::Ok as i32),
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::TransactionRequest(_)) => {
                Some(proto::server_originated_message::Submessage::TransactionResponse(
                    proto::TransactionResponse {
                        status: Some(proto::transaction_response::Status::Ok as i32),
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::VariableRequest(_)) => {
                Some(proto::server_originated_message::Submessage::VariableResponse(
                    proto::VariableResponse {
                        status: Some(proto::variable_response::Status::Ok as i32),
                        values: vec![r#""test_value""#.to_string()],
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::CloseRequest(_)) => {
                Some(proto::server_originated_message::Submessage::CloseResponse(
                    proto::CloseResponse {
                        statuses: vec![proto::close_response::Status::Ok as i32],
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::GetProfilePropertyRequest(_)) => {
                Some(proto::server_originated_message::Submessage::GetProfilePropertyResponse(
                    proto::GetProfilePropertyResponse {
                        status: Some(proto::get_profile_property_response::Status::Ok as i32),
                        properties: vec![proto::ProfileProperty {
                            key: Some("Name".to_string()),
                            json_value: Some(r#""Default""#.to_string()),
                        }],
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::SetProfilePropertyRequest(_)) => {
                Some(proto::server_originated_message::Submessage::SetProfilePropertyResponse(
                    proto::SetProfilePropertyResponse {
                        status: Some(proto::set_profile_property_response::Status::Ok as i32),
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::FocusRequest(_)) => {
                Some(proto::server_originated_message::Submessage::FocusResponse(
                    proto::FocusResponse {
                        notifications: vec![],
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::NotificationRequest(_)) => {
                Some(proto::server_originated_message::Submessage::NotificationResponse(
                    proto::NotificationResponse {
                        status: Some(proto::notification_response::Status::Ok as i32),
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::GetPropertyRequest(_)) => {
                Some(proto::server_originated_message::Submessage::GetPropertyResponse(
                    proto::GetPropertyResponse {
                        status: Some(proto::get_property_response::Status::Ok as i32),
                        json_value: Some(r#"{"width": 80, "height": 25}"#.to_string()),
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::SetPropertyRequest(_)) => {
                Some(proto::server_originated_message::Submessage::SetPropertyResponse(
                    proto::SetPropertyResponse {
                        status: Some(proto::set_property_response::Status::Ok as i32),
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::InjectRequest(_)) => {
                Some(proto::server_originated_message::Submessage::InjectResponse(
                    proto::InjectResponse {
                        status: vec![proto::inject_response::Status::Ok as i32],
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::RestartSessionRequest(_)) => {
                Some(proto::server_originated_message::Submessage::RestartSessionResponse(
                    proto::RestartSessionResponse {
                        status: Some(proto::restart_session_response::Status::Ok as i32),
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::GetPromptRequest(_)) => {
                Some(proto::server_originated_message::Submessage::GetPromptResponse(
                    proto::GetPromptResponse {
                        status: Some(proto::get_prompt_response::Status::Ok as i32),
                        prompt_range: None,
                        command_range: None,
                        output_range: None,
                        working_directory: Some("/tmp".to_string()),
                        command: Some("ls".to_string()),
                        prompt_state: Some(proto::get_prompt_response::State::Finished as i32),
                        exit_status: Some(0),
                        unique_prompt_id: Some("prompt-1".to_string()),
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::ListProfilesRequest(_)) => {
                Some(proto::server_originated_message::Submessage::ListProfilesResponse(
                    proto::ListProfilesResponse {
                        profiles: vec![proto::list_profiles_response::Profile {
                            properties: vec![proto::ProfileProperty {
                                key: Some("Name".to_string()),
                                json_value: Some(r#""Default""#.to_string()),
                            }],
                        }],
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::ColorPresetRequest(_)) => {
                Some(proto::server_originated_message::Submessage::ColorPresetResponse(
                    proto::ColorPresetResponse {
                        response: Some(proto::color_preset_response::Response::ListPresets(
                            proto::color_preset_response::ListPresets {
                                name: vec!["Solarized Dark".to_string(), "Tango Dark".to_string()],
                            },
                        )),
                        status: Some(proto::color_preset_response::Status::Ok as i32),
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::SavedArrangementRequest(_)) => {
                Some(proto::server_originated_message::Submessage::SavedArrangementResponse(
                    proto::SavedArrangementResponse {
                        status: Some(proto::saved_arrangement_response::Status::Ok as i32),
                        names: vec!["Default".to_string(), "Work".to_string()],
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::GetBroadcastDomainsRequest(_)) => {
                Some(proto::server_originated_message::Submessage::GetBroadcastDomainsResponse(
                    proto::GetBroadcastDomainsResponse {
                        broadcast_domains: vec![proto::BroadcastDomain {
                            session_ids: vec!["s1".to_string(), "s2".to_string()],
                        }],
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::MenuItemRequest(_)) => {
                Some(proto::server_originated_message::Submessage::MenuItemResponse(
                    proto::MenuItemResponse {
                        status: Some(proto::menu_item_response::Status::Ok as i32),
                        checked: Some(false),
                        enabled: Some(true),
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::ListPromptsRequest(_)) => {
                Some(proto::server_originated_message::Submessage::ListPromptsResponse(
                    proto::ListPromptsResponse {
                        status: Some(proto::list_prompts_response::Status::Ok as i32),
                        unique_prompt_id: vec!["p1".to_string(), "p2".to_string()],
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::SelectionRequest(_)) => {
                Some(proto::server_originated_message::Submessage::SelectionResponse(
                    proto::SelectionResponse {
                        status: Some(proto::selection_response::Status::Ok as i32),
                        response: Some(proto::selection_response::Response::GetSelectionResponse(
                            proto::selection_response::GetSelectionResponse {
                                selection: Some(proto::Selection {
                                    sub_selections: vec![],
                                }),
                            },
                        )),
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::SetBroadcastDomainsRequest(_)) => {
                Some(proto::server_originated_message::Submessage::SetBroadcastDomainsResponse(
                    proto::SetBroadcastDomainsResponse {
                        status: Some(proto::set_broadcast_domains_response::Status::Ok as i32),
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::TmuxRequest(_)) => {
                Some(proto::server_originated_message::Submessage::TmuxResponse(
                    proto::TmuxResponse {
                        payload: Some(proto::tmux_response::Payload::ListConnections(
                            proto::tmux_response::ListConnections {
                                connections: vec![],
                            },
                        )),
                        status: Some(proto::tmux_response::Status::Ok as i32),
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::ReorderTabsRequest(_)) => {
                Some(proto::server_originated_message::Submessage::ReorderTabsResponse(
                    proto::ReorderTabsResponse {
                        status: Some(proto::reorder_tabs_response::Status::Ok as i32),
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::PreferencesRequest(_)) => {
                Some(proto::server_originated_message::Submessage::PreferencesResponse(
                    proto::PreferencesResponse {
                        results: vec![proto::preferences_response::Result {
                            result: Some(proto::preferences_response::result::Result::GetPreferenceResult(
                                proto::preferences_response::result::GetPreferenceResult {
                                    json_value: Some(r#""value""#.to_string()),
                                },
                            )),
                        }],
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::RegisterToolRequest(_)) => {
                Some(proto::server_originated_message::Submessage::RegisterToolResponse(
                    proto::RegisterToolResponse {
                        status: Some(proto::register_tool_response::Status::Ok as i32),
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::StatusBarComponentRequest(_)) => {
                Some(proto::server_originated_message::Submessage::StatusBarComponentResponse(
                    proto::StatusBarComponentResponse {
                        status: Some(proto::status_bar_component_response::Status::Ok as i32),
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::InvokeFunctionRequest(_)) => {
                Some(proto::server_originated_message::Submessage::InvokeFunctionResponse(
                    proto::InvokeFunctionResponse {
                        disposition: Some(proto::invoke_function_response::Disposition::Success(
                            proto::invoke_function_response::Success {
                                json_result: Some(r#""ok""#.to_string()),
                            },
                        )),
                    },
                ))
            }
            Some(proto::client_originated_message::Submessage::ServerOriginatedRpcResultRequest(_)) => {
                Some(proto::server_originated_message::Submessage::ServerOriginatedRpcResultResponse(
                    proto::ServerOriginatedRpcResultResponse {},
                ))
            }
            Some(proto::client_originated_message::Submessage::SetTabLayoutRequest(_)) => {
                Some(proto::server_originated_message::Submessage::SetTabLayoutResponse(
                    proto::SetTabLayoutResponse {
                        status: Some(proto::set_tab_layout_response::Status::Ok as i32),
                    },
                ))
            }
            _ => None,
        };

        submessage.map(|s| proto::ServerOriginatedMessage {
            id: req.id,
            submessage: Some(s),
        })
    })
}
