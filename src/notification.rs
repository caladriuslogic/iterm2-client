use crate::proto;
use futures_util::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::broadcast;

pub struct NotificationStream {
    rx: broadcast::Receiver<proto::Notification>,
}

impl NotificationStream {
    pub fn new(rx: broadcast::Receiver<proto::Notification>) -> Self {
        Self { rx }
    }
}

impl Stream for NotificationStream {
    type Item = proto::Notification;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match self.rx.try_recv() {
                Ok(notif) => return Poll::Ready(Some(notif)),
                Err(broadcast::error::TryRecvError::Empty) => {
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }
                Err(broadcast::error::TryRecvError::Lagged(_)) => {
                    // Messages were dropped. Return Pending and retry on
                    // next poll to avoid busy-spinning.
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }
                Err(broadcast::error::TryRecvError::Closed) => return Poll::Ready(None),
            }
        }
    }
}

/// Helper to create a typed notification stream that filters for specific notification types.
pub fn keystroke_notifications(
    rx: broadcast::Receiver<proto::Notification>,
) -> impl Stream<Item = proto::KeystrokeNotification> {
    futures_util::stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(notif) => {
                    if let Some(k) = notif.keystroke_notification {
                        return Some((k, rx));
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    })
}

pub fn screen_update_notifications(
    rx: broadcast::Receiver<proto::Notification>,
) -> impl Stream<Item = proto::ScreenUpdateNotification> {
    futures_util::stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(notif) => {
                    if let Some(n) = notif.screen_update_notification {
                        return Some((n, rx));
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    })
}

pub fn prompt_notifications(
    rx: broadcast::Receiver<proto::Notification>,
) -> impl Stream<Item = proto::PromptNotification> {
    futures_util::stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(notif) => {
                    if let Some(n) = notif.prompt_notification {
                        return Some((n, rx));
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    })
}

pub fn new_session_notifications(
    rx: broadcast::Receiver<proto::Notification>,
) -> impl Stream<Item = proto::NewSessionNotification> {
    futures_util::stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(notif) => {
                    if let Some(n) = notif.new_session_notification {
                        return Some((n, rx));
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    })
}

pub fn terminate_session_notifications(
    rx: broadcast::Receiver<proto::Notification>,
) -> impl Stream<Item = proto::TerminateSessionNotification> {
    futures_util::stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(notif) => {
                    if let Some(n) = notif.terminate_session_notification {
                        return Some((n, rx));
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    })
}

pub fn focus_changed_notifications(
    rx: broadcast::Receiver<proto::Notification>,
) -> impl Stream<Item = proto::FocusChangedNotification> {
    futures_util::stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(notif) => {
                    if let Some(n) = notif.focus_changed_notification {
                        return Some((n, rx));
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    })
}

pub fn layout_changed_notifications(
    rx: broadcast::Receiver<proto::Notification>,
) -> impl Stream<Item = proto::LayoutChangedNotification> {
    futures_util::stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(notif) => {
                    if let Some(n) = notif.layout_changed_notification {
                        return Some((n, rx));
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    })
}

pub fn variable_changed_notifications(
    rx: broadcast::Receiver<proto::Notification>,
) -> impl Stream<Item = proto::VariableChangedNotification> {
    futures_util::stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(notif) => {
                    if let Some(n) = notif.variable_changed_notification {
                        return Some((n, rx));
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    })
}

pub fn custom_escape_sequence_notifications(
    rx: broadcast::Receiver<proto::Notification>,
) -> impl Stream<Item = proto::CustomEscapeSequenceNotification> {
    futures_util::stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(notif) => {
                    if let Some(n) = notif.custom_escape_sequence_notification {
                        return Some((n, rx));
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    })
}
