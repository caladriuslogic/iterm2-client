//! Notification streams with typed filtering.
//!
//! Use `NotificationStream` for raw notifications, or typed helpers like
//! `keystroke_notifications` and `new_session_notifications` to filter for
//! specific event types.

use crate::proto;
use futures_util::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::broadcast;

/// A stream of all iTerm2 notifications (unfiltered).
///
/// Wraps a `broadcast::Receiver` and implements `futures_util::Stream`.
pub struct NotificationStream {
    rx: broadcast::Receiver<proto::Notification>,
}

impl NotificationStream {
    /// Create a new notification stream from a broadcast receiver.
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

/// Stream of keystroke notifications, filtering out all other types.
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

/// Stream of screen update notifications.
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

/// Stream of prompt notifications (prompt shown, command started, command ended).
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

/// Stream of new session creation notifications.
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

/// Stream of session termination notifications.
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

/// Stream of focus change notifications (window, tab, session, or app focus).
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

/// Stream of layout change notifications (windows/tabs/sessions restructured).
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

/// Stream of variable change notifications.
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

/// Stream of custom escape sequence notifications (OSC 1337).
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
