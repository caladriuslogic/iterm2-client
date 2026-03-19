use iterm2_client::proto;
use std::sync::Arc;

use crate::common::mock_server::{self, MockServer};

// === App tests ===

#[tokio::test]
async fn app_list_sessions_parses_tree() {
    let server = MockServer::start(mock_server::list_sessions_handler()).await;
    let conn = mock_server::connect_to_mock(server.addr).await;
    let app = iterm2_client::App::new(conn);

    let result = app.list_sessions().await.unwrap();
    assert_eq!(result.windows.len(), 1);
    assert_eq!(result.windows[0].window.id, "window-1");
    assert_eq!(result.windows[0].tabs.len(), 1);
    assert_eq!(result.windows[0].tabs[0].tab.id, "tab-1");
    assert_eq!(result.windows[0].tabs[0].sessions.len(), 1);
    assert_eq!(result.windows[0].tabs[0].sessions[0].id, "session-1");
    assert_eq!(
        result.windows[0].tabs[0].sessions[0].title.as_deref(),
        Some("bash")
    );

    server.shutdown().await;
}

#[tokio::test]
async fn app_create_tab() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = mock_server::connect_to_mock(server.addr).await;
    let app = iterm2_client::App::new(conn);

    let result = app.create_tab(None, None).await.unwrap();
    assert_eq!(result.session.id, "new-session-1");
    assert_eq!(result.tab.id, "42");
    assert_eq!(result.window.id, "window-1");

    server.shutdown().await;
}

#[tokio::test]
async fn app_create_tab_with_profile() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = mock_server::connect_to_mock(server.addr).await;
    let app = iterm2_client::App::new(conn);

    let result = app.create_tab(Some("Default"), Some("window-1")).await.unwrap();
    assert_eq!(result.session.id, "new-session-1");

    server.shutdown().await;
}

#[tokio::test]
async fn app_focus() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = mock_server::connect_to_mock(server.addr).await;
    let app = iterm2_client::App::new(conn);

    let notifications = app.focus().await.unwrap();
    assert!(notifications.is_empty());

    server.shutdown().await;
}

#[tokio::test]
async fn app_activate() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = mock_server::connect_to_mock(server.addr).await;
    let app = iterm2_client::App::new(conn);

    app.activate(true, false).await.unwrap();

    server.shutdown().await;
}

#[tokio::test]
async fn app_begin_end_transaction() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = mock_server::connect_to_mock(server.addr).await;
    let app = iterm2_client::App::new(conn);

    app.begin_transaction().await.unwrap();
    app.end_transaction().await.unwrap();

    server.shutdown().await;
}

#[tokio::test]
async fn app_list_profiles() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = mock_server::connect_to_mock(server.addr).await;
    let app = iterm2_client::App::new(conn);

    let resp = app.list_profiles(vec![], vec![]).await.unwrap();
    assert_eq!(resp.profiles.len(), 1);
    assert_eq!(resp.profiles[0].properties[0].key.as_deref(), Some("Name"));

    server.shutdown().await;
}

#[tokio::test]
async fn app_list_color_presets() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = mock_server::connect_to_mock(server.addr).await;
    let app = iterm2_client::App::new(conn);

    let presets = app.list_color_presets().await.unwrap();
    assert_eq!(presets, vec!["Solarized Dark", "Tango Dark"]);

    server.shutdown().await;
}

#[tokio::test]
async fn app_list_arrangements() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = mock_server::connect_to_mock(server.addr).await;
    let app = iterm2_client::App::new(conn);

    let arrangements = app.list_arrangements().await.unwrap();
    assert_eq!(arrangements, vec!["Default", "Work"]);

    server.shutdown().await;
}

#[tokio::test]
async fn app_get_broadcast_domains() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = mock_server::connect_to_mock(server.addr).await;
    let app = iterm2_client::App::new(conn);

    let domains = app.get_broadcast_domains().await.unwrap();
    assert_eq!(domains.len(), 1);
    assert_eq!(domains[0].session_ids, vec!["s1", "s2"]);

    server.shutdown().await;
}

#[tokio::test]
async fn app_subscribe_notifications() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = mock_server::connect_to_mock(server.addr).await;
    let app = iterm2_client::App::new(conn);

    let _rx = app.subscribe_notifications();
    // Just verify it doesn't panic

    server.shutdown().await;
}

#[tokio::test]
async fn app_connection_accessors() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = mock_server::connect_to_mock(server.addr).await;
    let app = iterm2_client::App::new(conn);

    let _conn_ref = app.connection();
    let _conn_arc = app.connection_arc();

    server.shutdown().await;
}

#[tokio::test]
async fn app_from_arc() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let app = iterm2_client::App::from_arc(conn);

    let _resp = app.focus().await.unwrap();

    server.shutdown().await;
}

// === Session tests ===

#[tokio::test]
async fn session_send_text_ok() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let session = iterm2_client::Session::new("s1".to_string(), None, conn).unwrap();

    session.send_text("hello\n").await.unwrap();

    server.shutdown().await;
}

#[tokio::test]
async fn session_get_screen_contents() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let session = iterm2_client::Session::new("s1".to_string(), None, conn).unwrap();

    let lines = session.get_screen_contents().await.unwrap();
    assert_eq!(lines, vec!["hello world"]);

    server.shutdown().await;
}

#[tokio::test]
async fn session_get_buffer_lines() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let session = iterm2_client::Session::new("s1".to_string(), None, conn).unwrap();

    let lines = session.get_buffer_lines(10).await.unwrap();
    assert_eq!(lines, vec!["hello world"]);

    server.shutdown().await;
}

#[tokio::test]
async fn session_split() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let session = iterm2_client::Session::new("s1".to_string(), None, conn).unwrap();

    let new_ids = session
        .split(
            proto::split_pane_request::SplitDirection::Vertical,
            false,
            None,
        )
        .await
        .unwrap();
    assert_eq!(new_ids, vec!["split-session-1"]);

    server.shutdown().await;
}

#[tokio::test]
async fn session_split_horizontal_with_profile() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let session = iterm2_client::Session::new("s1".to_string(), None, conn).unwrap();

    let new_ids = session
        .split(
            proto::split_pane_request::SplitDirection::Horizontal,
            true,
            Some("Default"),
        )
        .await
        .unwrap();
    assert_eq!(new_ids, vec!["split-session-1"]);

    server.shutdown().await;
}

#[tokio::test]
async fn session_get_variable() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let session = iterm2_client::Session::new("s1".to_string(), None, conn).unwrap();

    let val = session.get_variable("user.test").await.unwrap();
    assert_eq!(val.as_deref(), Some(r#""test_value""#));

    server.shutdown().await;
}

#[tokio::test]
async fn session_set_variable() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let session = iterm2_client::Session::new("s1".to_string(), None, conn).unwrap();

    session.set_variable("user.test", r#""hello""#).await.unwrap();

    server.shutdown().await;
}

#[tokio::test]
async fn session_get_profile_property() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let session = iterm2_client::Session::new("s1".to_string(), None, conn).unwrap();

    let props = session
        .get_profile_property(vec!["Name".to_string()])
        .await
        .unwrap();
    assert_eq!(props.len(), 1);
    assert_eq!(props[0].key.as_deref(), Some("Name"));

    server.shutdown().await;
}

#[tokio::test]
async fn session_set_profile_property() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let session = iterm2_client::Session::new("s1".to_string(), None, conn).unwrap();

    session
        .set_profile_property("Badge Text", r#""test""#)
        .await
        .unwrap();

    server.shutdown().await;
}

#[tokio::test]
async fn session_inject() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let session = iterm2_client::Session::new("s1".to_string(), None, conn).unwrap();

    session.inject(b"test data".to_vec()).await.unwrap();

    server.shutdown().await;
}

#[tokio::test]
async fn session_restart() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let session = iterm2_client::Session::new("s1".to_string(), None, conn).unwrap();

    session.restart(true).await.unwrap();

    server.shutdown().await;
}

#[tokio::test]
async fn session_close() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let session = iterm2_client::Session::new("s1".to_string(), None, conn).unwrap();

    session.close(true).await.unwrap();

    server.shutdown().await;
}

#[tokio::test]
async fn session_activate() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let session = iterm2_client::Session::new("s1".to_string(), None, conn).unwrap();

    session.activate().await.unwrap();

    server.shutdown().await;
}

#[tokio::test]
async fn session_get_prompt() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let session = iterm2_client::Session::new("s1".to_string(), None, conn).unwrap();

    let prompt = session.get_prompt().await.unwrap();
    assert_eq!(prompt.working_directory.as_deref(), Some("/tmp"));
    assert_eq!(prompt.command.as_deref(), Some("ls"));
    assert_eq!(prompt.exit_status, Some(0));
    assert_eq!(prompt.unique_prompt_id.as_deref(), Some("prompt-1"));

    server.shutdown().await;
}

#[tokio::test]
async fn session_connection_accessor() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let session = iterm2_client::Session::new("s1".to_string(), None, conn).unwrap();

    let _conn = session.connection();

    server.shutdown().await;
}

// === Tab tests ===

#[tokio::test]
async fn tab_activate() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let tab = iterm2_client::Tab::new("tab-1".to_string(), conn).unwrap();

    tab.activate().await.unwrap();

    server.shutdown().await;
}

#[tokio::test]
async fn tab_close() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let tab = iterm2_client::Tab::new("tab-1".to_string(), conn).unwrap();

    tab.close(true).await.unwrap();

    server.shutdown().await;
}

#[tokio::test]
async fn tab_get_variable() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let tab = iterm2_client::Tab::new("tab-1".to_string(), conn).unwrap();

    let val = tab.get_variable("user.test").await.unwrap();
    assert_eq!(val.as_deref(), Some(r#""test_value""#));

    server.shutdown().await;
}

#[tokio::test]
async fn tab_connection_accessor() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let tab = iterm2_client::Tab::new("tab-1".to_string(), conn).unwrap();

    let _conn = tab.connection();

    server.shutdown().await;
}

// === Window tests ===

#[tokio::test]
async fn window_create_tab() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let window = iterm2_client::Window::new("w1".to_string(), conn).unwrap();

    let result = window.create_tab(None).await.unwrap();
    assert_eq!(result.session_id.as_deref(), Some("new-session-1"));
    assert_eq!(result.tab_id.as_deref(), Some("42"));

    server.shutdown().await;
}

#[tokio::test]
async fn window_create_tab_with_profile() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let window = iterm2_client::Window::new("w1".to_string(), conn).unwrap();

    let result = window.create_tab(Some("Default")).await.unwrap();
    assert!(result.session_id.is_some());

    server.shutdown().await;
}

#[tokio::test]
async fn window_activate() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let window = iterm2_client::Window::new("w1".to_string(), conn).unwrap();

    window.activate().await.unwrap();

    server.shutdown().await;
}

#[tokio::test]
async fn window_close() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let window = iterm2_client::Window::new("w1".to_string(), conn).unwrap();

    window.close(true).await.unwrap();

    server.shutdown().await;
}

#[tokio::test]
async fn window_get_set_property() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let window = iterm2_client::Window::new("w1".to_string(), conn).unwrap();

    let val = window.get_property("frame").await.unwrap();
    assert!(val.is_some());

    window.set_property("fullscreen", "true").await.unwrap();

    server.shutdown().await;
}

#[tokio::test]
async fn window_get_variable() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let window = iterm2_client::Window::new("w1".to_string(), conn).unwrap();

    let val = window.get_variable("user.test").await.unwrap();
    assert_eq!(val.as_deref(), Some(r#""test_value""#));

    server.shutdown().await;
}

#[tokio::test]
async fn window_connection_accessor() {
    let server = MockServer::start(mock_server::echo_ok_handler()).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let window = iterm2_client::Window::new("w1".to_string(), conn).unwrap();

    let _conn = window.connection();

    server.shutdown().await;
}

// === Error path tests ===

#[tokio::test]
async fn non_ok_status_returns_error() {
    let handler: mock_server::Handler = Arc::new(|req| {
        Some(proto::ServerOriginatedMessage {
            id: req.id,
            submessage: Some(
                proto::server_originated_message::Submessage::SendTextResponse(
                    proto::SendTextResponse {
                        status: Some(proto::send_text_response::Status::SessionNotFound as i32),
                    },
                ),
            ),
        })
    });

    let server = MockServer::start(handler).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let session = iterm2_client::Session::new("bad-session".to_string(), None, conn).unwrap();

    let err = session.send_text("hello").await.unwrap_err();
    assert!(err.to_string().contains("status"), "got: {err}");

    server.shutdown().await;
}

#[tokio::test]
async fn unexpected_response_returns_error() {
    // Return a ListSessionsResponse when SendText is expected
    let handler: mock_server::Handler = Arc::new(|req| {
        Some(proto::ServerOriginatedMessage {
            id: req.id,
            submessage: Some(
                proto::server_originated_message::Submessage::ListSessionsResponse(
                    proto::ListSessionsResponse {
                        windows: vec![],
                        buried_sessions: vec![],
                    },
                ),
            ),
        })
    });

    let server = MockServer::start(handler).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let session = iterm2_client::Session::new("s1".to_string(), None, conn).unwrap();

    let err = session.send_text("hello").await.unwrap_err();
    assert!(
        err.to_string().contains("Unexpected response"),
        "got: {err}"
    );

    server.shutdown().await;
}

#[tokio::test]
async fn inject_non_ok_status_returns_error() {
    let handler: mock_server::Handler = Arc::new(|req| {
        Some(proto::ServerOriginatedMessage {
            id: req.id,
            submessage: Some(
                proto::server_originated_message::Submessage::InjectResponse(
                    proto::InjectResponse {
                        status: vec![proto::inject_response::Status::SessionNotFound as i32],
                    },
                ),
            ),
        })
    });

    let server = MockServer::start(handler).await;
    let conn = Arc::new(mock_server::connect_to_mock(server.addr).await);
    let session = iterm2_client::Session::new("s1".to_string(), None, conn).unwrap();

    let err = session.inject(b"data".to_vec()).await.unwrap_err();
    assert!(err.to_string().contains("Inject failed"), "got: {err}");

    server.shutdown().await;
}
