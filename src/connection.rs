use crate::auth::{self, AppleScriptRunner, Credentials, OsascriptRunner};
use crate::error::{Error, Result};
use crate::proto;
use crate::transport;
use futures_util::{SinkExt, StreamExt};
use prost::Message;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{broadcast, oneshot, Mutex};
use tokio_tungstenite::tungstenite;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const NOTIFICATION_CHANNEL_SIZE: usize = 1024;
const MAX_PENDING_REQUESTS: usize = 4096;

type PendingMap = HashMap<i64, oneshot::Sender<proto::ServerOriginatedMessage>>;

pub struct Connection<S> {
    inner: Arc<Inner<S>>,
    shared: Arc<Shared>,
}

struct Inner<S> {
    sink: Mutex<transport::WsSink<S>>,
    _dispatch_handle: tokio::task::JoinHandle<()>,
}

struct Shared {
    pending: Mutex<PendingMap>,
    notification_tx: broadcast::Sender<proto::Notification>,
    next_id: AtomicI64,
}

impl<S> Clone for Connection<S> {
    fn clone(&self) -> Self {
        Connection {
            inner: Arc::clone(&self.inner),
            shared: Arc::clone(&self.shared),
        }
    }
}

impl Connection<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>> {
    pub async fn connect(app_name: &str) -> Result<Self> {
        Self::connect_with_runner(app_name, &OsascriptRunner).await
    }

    pub async fn connect_with_runner(
        app_name: &str,
        runner: &dyn AppleScriptRunner,
    ) -> Result<Self> {
        let credentials = auth::resolve_credentials(runner)?;
        Self::connect_with_credentials(app_name, &credentials).await
    }

    pub async fn connect_with_credentials(
        app_name: &str,
        credentials: &Credentials,
    ) -> Result<Self> {
        let (sink, source) = transport::connect(credentials, app_name).await?;
        Ok(Self::from_split(sink, source))
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin + Send + 'static> Connection<S> {
    pub fn from_split(sink: transport::WsSink<S>, source: transport::WsSource<S>) -> Self {
        let (notification_tx, _) = broadcast::channel(NOTIFICATION_CHANNEL_SIZE);
        let shared = Arc::new(Shared {
            pending: Mutex::new(HashMap::new()),
            notification_tx: notification_tx.clone(),
            next_id: AtomicI64::new(1),
        });

        let shared_for_dispatch = Arc::clone(&shared);
        let dispatch_handle = tokio::spawn(dispatch_loop(source, shared_for_dispatch));

        let inner = Arc::new(Inner {
            sink: Mutex::new(sink),
            _dispatch_handle: dispatch_handle,
        });

        Connection { inner, shared }
    }

    pub async fn call(
        &self,
        request: proto::ClientOriginatedMessage,
    ) -> Result<proto::ServerOriginatedMessage> {
        self.call_with_timeout(request, DEFAULT_TIMEOUT).await
    }

    pub async fn call_with_timeout(
        &self,
        mut request: proto::ClientOriginatedMessage,
        timeout: Duration,
    ) -> Result<proto::ServerOriginatedMessage> {
        let id = self.shared.next_id.fetch_add(1, Ordering::SeqCst);

        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.shared.pending.lock().await;
            // Prevent unbounded growth of the pending map
            if pending.len() >= MAX_PENDING_REQUESTS {
                return Err(Error::Api(
                    "Too many pending requests (max 4096)".to_string(),
                ));
            }
            pending.insert(id, tx);
        }

        request.id = Some(id);

        // Encode and send
        let mut buf = Vec::new();
        request
            .encode(&mut buf)
            .map_err(|e| Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        let send_result = {
            let mut sink = self.inner.sink.lock().await;
            SinkExt::<tungstenite::Message>::send(
                &mut *sink,
                tungstenite::Message::Binary(buf.into()),
            )
            .await
        };

        if let Err(e) = send_result {
            // Clean up pending entry on send failure
            let mut pending = self.shared.pending.lock().await;
            pending.remove(&id);
            return Err(Error::WebSocket(e));
        }

        // Wait for response with timeout.
        // The oneshot receiver is the sole owner — if the dispatch loop sends
        // on it after our timeout fires, the send simply fails (receiver dropped).
        // This avoids TOCTOU: we don't need to manually clean up on timeout
        // because dropping `rx` causes the dispatch loop's `sender.send()` to
        // return Err, which is already handled with `let _ =`.
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(response)) => {
                // Check for error submessage
                if let Some(proto::server_originated_message::Submessage::Error(err_str)) =
                    &response.submessage
                {
                    return Err(Error::Api(err_str.clone()));
                }
                Ok(response)
            }
            Ok(Err(_)) => {
                // Sender was dropped (dispatch loop ended) — clean up
                let mut pending = self.shared.pending.lock().await;
                pending.remove(&id);
                Err(Error::ConnectionClosed)
            }
            Err(_) => {
                // Timeout — clean up the pending entry so it doesn't leak.
                // There's a benign race: dispatch_loop may have already removed
                // and sent on the oneshot, but since we're dropping rx here the
                // response is simply discarded. No data corruption is possible.
                let mut pending = self.shared.pending.lock().await;
                pending.remove(&id);
                Err(Error::Timeout(timeout))
            }
        }
    }

    pub fn subscribe_notifications(&self) -> broadcast::Receiver<proto::Notification> {
        self.shared.notification_tx.subscribe()
    }
}

async fn dispatch_loop<S: AsyncRead + AsyncWrite + Unpin>(
    mut source: transport::WsSource<S>,
    shared: Arc<Shared>,
) {
    let mut decode_errors: u32 = 0;
    const MAX_CONSECUTIVE_DECODE_ERRORS: u32 = 100;

    while let Some(msg_result) = source.next().await {
        let msg = match msg_result {
            Ok(tungstenite::Message::Binary(data)) => {
                match proto::ServerOriginatedMessage::decode(data.as_ref()) {
                    Ok(m) => {
                        decode_errors = 0;
                        m
                    }
                    Err(_) => {
                        decode_errors += 1;
                        if decode_errors >= MAX_CONSECUTIVE_DECODE_ERRORS {
                            // Too many consecutive decode errors — likely a protocol
                            // mismatch or corrupted connection. Break to avoid CPU spin.
                            break;
                        }
                        continue;
                    }
                }
            }
            Ok(tungstenite::Message::Close(_)) => break,
            Ok(_) => continue,
            Err(_) => break,
        };

        // Notification: no id set, notification submessage
        if msg.id.is_none() {
            if let Some(proto::server_originated_message::Submessage::Notification(notif)) =
                msg.submessage
            {
                let _ = shared.notification_tx.send(notif);
            }
            continue;
        }

        if let Some(id) = msg.id {
            let mut pending = shared.pending.lock().await;
            if let Some(sender) = pending.remove(&id) {
                let _ = sender.send(msg);
            }
        }
    }
}
