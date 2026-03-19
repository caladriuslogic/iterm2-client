use iterm2_client::proto;
use std::sync::Arc;

/// These tests require a running iTerm2 instance.
/// Run with: cargo test -- --ignored

#[tokio::test]
#[ignore]
async fn connect_and_list_sessions() {
    let conn = iterm2_client::Connection::connect("iterm2-client-test")
        .await
        .unwrap();
    let app = iterm2_client::App::new(conn);

    let result = app.list_sessions().await.unwrap();
    assert!(
        !result.windows.is_empty(),
        "Expected at least one window in iTerm2"
    );
    for w in &result.windows {
        assert!(!w.tabs.is_empty(), "Expected at least one tab per window");
        for t in &w.tabs {
            assert!(
                !t.sessions.is_empty(),
                "Expected at least one session per tab"
            );
        }
    }
}

#[tokio::test]
#[ignore]
async fn send_text_and_read_buffer() {
    let conn = iterm2_client::Connection::connect("iterm2-client-test")
        .await
        .unwrap();
    let app = iterm2_client::App::new(conn);

    let result = app.list_sessions().await.unwrap();
    let session = &result.windows[0].tabs[0].sessions[0];

    // Send some unique text
    let marker = format!("ITERM2_TEST_{}", std::process::id());
    session
        .send_text(&format!("echo {marker}\n"))
        .await
        .unwrap();

    // Wait a bit for output
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let lines = session.get_buffer_lines(50).await.unwrap();
    assert!(
        lines.iter().any(|l| l.contains(&marker)),
        "Expected to find marker '{marker}' in buffer, got: {lines:?}"
    );
}

#[tokio::test]
#[ignore]
async fn create_tab_and_close() {
    let conn = iterm2_client::Connection::connect("iterm2-client-test")
        .await
        .unwrap();
    let app = iterm2_client::App::new(conn);

    let result = app.create_tab(None, None).await.unwrap();
    assert!(!result.session.id.is_empty());

    // Clean up
    result.session.close(true).await.unwrap();
}

#[tokio::test]
#[ignore]
async fn split_pane_and_close() {
    let conn = iterm2_client::Connection::connect("iterm2-client-test")
        .await
        .unwrap();
    let conn = Arc::new(conn);
    let app = iterm2_client::App::from_arc(Arc::clone(&conn));

    let result = app.list_sessions().await.unwrap();
    let session = &result.windows[0].tabs[0].sessions[0];

    let new_ids = session
        .split(
            proto::split_pane_request::SplitDirection::Vertical,
            false,
            None,
        )
        .await
        .unwrap();
    assert!(!new_ids.is_empty());

    // Clean up the new session
    let new_session =
        iterm2_client::Session::new(new_ids[0].clone(), None, Arc::clone(&conn)).unwrap();
    new_session.close(true).await.unwrap();
}

#[tokio::test]
#[ignore]
async fn get_set_variables() {
    let conn = iterm2_client::Connection::connect("iterm2-client-test")
        .await
        .unwrap();
    let conn = Arc::new(conn);
    let app = iterm2_client::App::from_arc(Arc::clone(&conn));

    let result = app.list_sessions().await.unwrap();
    let session = &result.windows[0].tabs[0].sessions[0];

    session
        .set_variable("user.iterm2_client_test", r#""hello""#)
        .await
        .unwrap();

    let val = session
        .get_variable("user.iterm2_client_test")
        .await
        .unwrap();
    assert_eq!(val.as_deref(), Some(r#""hello""#));
}

#[tokio::test]
#[ignore]
async fn subscribe_notifications() {
    let conn = iterm2_client::Connection::connect("iterm2-client-test")
        .await
        .unwrap();
    let conn = Arc::new(conn);
    let app = iterm2_client::App::from_arc(Arc::clone(&conn));

    // Subscribe to new session notifications
    let mut rx = app.subscribe_notifications();

    conn.call(iterm2_client::request::subscribe_notification(
        proto::NotificationType::NotifyOnNewSession,
        None,
    ))
    .await
    .unwrap();

    // Create a tab to trigger the notification
    let result = app.create_tab(None, None).await.unwrap();

    // Wait for notification
    let notif = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
        .await
        .expect("Timed out waiting for notification")
        .unwrap();
    assert!(notif.new_session_notification.is_some());

    // Clean up
    result.session.close(true).await.unwrap();
}
