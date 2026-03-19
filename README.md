# iterm2-client

Rust client for the [iTerm2](https://iterm2.com/) scripting API.

Communicates over WebSocket + Protobuf to control iTerm2 windows, tabs, and sessions programmatically. Covers all 34 operations in the iTerm2 API.

## Quick start

```rust
use iterm2_client::{App, Connection};

#[tokio::main]
async fn main() -> iterm2_client::Result<()> {
    // Connect to iTerm2 (reads ITERM2_COOKIE/ITERM2_KEY from env,
    // falls back to osascript)
    let conn = Connection::connect("my-app").await?;
    let app = App::new(conn);

    // List all sessions
    let result = app.list_sessions().await?;
    for window in &result.windows {
        println!("Window: {}", window.window.id);
        for tab in &window.tabs {
            println!("  Tab: {}", tab.tab.id);
            for session in &tab.sessions {
                println!("    Session: {} ({:?})", session.id, session.title);
            }
        }
    }

    // Send text to the first session
    let session = &result.windows[0].tabs[0].sessions[0];
    session.send_text("echo hello from Rust\n").await?;

    Ok(())
}
```

## Authentication

iTerm2 requires a cookie and key for API access. The client resolves credentials in order:

1. `ITERM2_COOKIE` and `ITERM2_KEY` environment variables
2. AppleScript request to iTerm2 via `osascript` (prompts user for permission on first use)

## Architecture

### Two-layer API

**Low-level** -- `Connection::call()` sends raw protobuf `ClientOriginatedMessage` and returns `ServerOriginatedMessage`. Use this for operations not covered by the high-level API, or when you need full control over the request/response.

```rust
use iterm2_client::{Connection, request, proto};

let conn = Connection::connect("my-app").await?;
let resp = conn.call(request::list_sessions()).await?;
```

**High-level** -- `App`, `Window`, `Tab`, and `Session` types wrap `Arc<Connection>` and provide ergonomic methods with status checking.

```rust
use iterm2_client::{App, Connection};

let app = App::new(Connection::connect("my-app").await?);
let session = &app.list_sessions().await?.windows[0].tabs[0].sessions[0];
session.send_text("ls\n").await?;
let lines = session.get_screen_contents().await?;
```

### Notifications

Subscribe to iTerm2 events (new sessions, keystrokes, focus changes, etc.):

```rust
use iterm2_client::{request, proto, notification};
use futures_util::StreamExt;

// Subscribe at the protocol level
conn.call(request::subscribe_notification(
    proto::NotificationType::NotifyOnNewSession,
    None,
)).await?;

// Receive typed notifications
let rx = conn.subscribe_notifications();
let mut stream = Box::pin(notification::new_session_notifications(rx));
while let Some(notif) = stream.next().await {
    println!("New session: {:?}", notif.session_id);
}
```

## API coverage

All 34 iTerm2 API operations have request builders in the `request` module:

| Category | Operations |
|----------|-----------|
| Sessions | `list_sessions`, `send_text`, `get_buffer`, `get_prompt`, `list_prompts`, `split_pane`, `inject`, `restart_session`, `close_sessions` |
| Tabs | `create_tab`, `close_tabs`, `set_tab_layout`, `reorder_tabs` |
| Windows | `close_windows`, `get_property`, `set_property` |
| Activation | `activate_session`, `activate_tab`, `activate_window`, `activate_app` |
| Variables | `get_variable_session`, `set_variable_session`, `get_variable_tab`, `get_variable_window`, `get_variable_app` |
| Profiles | `get_profile_property`, `set_profile_property`, `list_profiles` |
| Notifications | `subscribe_notification`, `unsubscribe_notification` |
| Other | `focus`, `begin_transaction`, `end_transaction`, `saved_arrangement`, `color_presets`, `broadcast_domains`, `tmux`, `preferences`, `register_tool`, `invoke_function`, `selection`, `menu_item`, `status_bar_component`, `rpc_result` |

## Testing

```bash
# Unit tests (mock WebSocket server, no iTerm2 needed)
cargo test

# Integration tests (requires running iTerm2)
cargo test -- --ignored

# Coverage report (requires cargo-llvm-cov)
cargo llvm-cov --lib --tests test --ignore-filename-regex "proto\.rs" --html
```

## License

GPL-2.0-only (matching iTerm2)
