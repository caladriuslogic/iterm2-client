//! WebSocket transport for connecting to iTerm2.
//!
//! Supports Unix socket (default) and legacy TCP (`ws://localhost:1912`) connections.
//! Use `connect` for the default Unix socket transport, or `connect_tcp` for
//! legacy TCP connections.

use crate::auth::Credentials;
use crate::error::{Error, Result};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::StreamExt;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::header::HeaderValue;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

pub type WsStream<S> = WebSocketStream<S>;
pub type WsSink<S> = SplitSink<WsStream<S>, tokio_tungstenite::tungstenite::Message>;
pub type WsSource<S> = SplitStream<WsStream<S>>;

const SUBPROTOCOL: &str = "api.iterm2.com";
const TCP_URL: &str = "ws://localhost:1912";
fn unix_socket_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    std::path::PathBuf::from(home)
        .join("Library/Application Support/iTerm2/private/socket")
}

/// Connect to iTerm2 over TCP WebSocket at `ws://localhost:1912`.
pub async fn connect_tcp(
    credentials: &Credentials,
    app_name: &str,
) -> Result<(WsSink<MaybeTlsStream<tokio::net::TcpStream>>, WsSource<MaybeTlsStream<tokio::net::TcpStream>>)> {
    let mut request = TCP_URL.into_client_request()?;
    apply_headers(request.headers_mut(), credentials, app_name)?;
    let config = ws_config();
    let (ws_stream, _response) =
        tokio_tungstenite::connect_async_with_config(request, Some(config), false).await?;
    Ok(ws_stream.split())
}

/// Connect to iTerm2 over a Unix domain socket.
pub async fn connect_unix(
    credentials: &Credentials,
    app_name: &str,
) -> Result<(WsSink<tokio::net::UnixStream>, WsSource<tokio::net::UnixStream>)> {
    let path = unix_socket_path();
    let stream = tokio::net::UnixStream::connect(&path).await?;
    connect_with_stream(stream, credentials, app_name).await
}

/// Upgrade an existing `AsyncRead + AsyncWrite` stream to a WebSocket connection.
///
/// Useful for testing with mock streams or custom transports.
pub async fn connect_with_stream<S>(
    stream: S,
    credentials: &Credentials,
    app_name: &str,
) -> Result<(WsSink<S>, WsSource<S>)>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut request = TCP_URL.into_client_request()?;
    apply_headers(request.headers_mut(), credentials, app_name)?;
    let config = ws_config();
    let (ws_stream, _response) =
        tokio_tungstenite::client_async_with_config(request, stream, Some(config)).await?;
    Ok(ws_stream.split())
}

/// Connect to iTerm2 using the default Unix socket transport.
///
/// iTerm2 only serves its API over a Unix domain socket at
/// `~/Library/Application Support/iTerm2/private/socket`.
/// TCP on port 1912 is legacy and no longer served.
pub async fn connect(
    credentials: &Credentials,
    app_name: &str,
) -> Result<(WsSink<tokio::net::UnixStream>, WsSource<tokio::net::UnixStream>)> {
    connect_unix(credentials, app_name).await
}

fn make_header_value(value: &str, field_name: &str) -> Result<HeaderValue> {
    HeaderValue::from_str(value).map_err(|_| {
        Error::Auth(format!(
            "Invalid characters in {field_name} (must be visible ASCII)"
        ))
    })
}

fn apply_headers(
    headers: &mut tokio_tungstenite::tungstenite::http::HeaderMap,
    credentials: &Credentials,
    app_name: &str,
) -> Result<()> {
    headers.insert(
        "Sec-WebSocket-Protocol",
        HeaderValue::from_static(SUBPROTOCOL),
    );
    headers.insert(
        "Origin",
        HeaderValue::from_static("ws://localhost"),
    );
    headers.insert(
        "x-iterm2-library-version",
        HeaderValue::from_static(concat!("rust ", "0.2.1")),
    );
    headers.insert(
        "x-iterm2-cookie",
        make_header_value(&credentials.cookie, "cookie")?,
    );
    headers.insert(
        "x-iterm2-key",
        make_header_value(&credentials.key, "key")?,
    );
    headers.insert(
        "x-iterm2-advisory-name",
        make_header_value(app_name, "app_name")?,
    );
    Ok(())
}

fn ws_config() -> WebSocketConfig {
    let mut config = WebSocketConfig::default();
    config.max_frame_size = Some(4 * 1024 * 1024); // 4MB per frame
    config.max_message_size = Some(8 * 1024 * 1024); // 8MB total
    config
}
