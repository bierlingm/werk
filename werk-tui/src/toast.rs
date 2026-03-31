//! Toast notification system — replaces TransientMessage.
//!
//! Wraps ftui's NotificationQueue with convenience methods for gesture feedback.
//! Toasts auto-dismiss after 3 seconds, stack in bottom-right, max 3 visible.

use std::time::Duration;

use ftui::widgets::{
    NotificationQueue, QueueConfig, Toast, ToastIcon, ToastPosition,
    ToastAction, ToastStyle, Widget,
    notification_queue::NotificationStack,
};
use ftui::layout::Rect;
use ftui::Frame;

/// Toast queue state — lives on InstrumentApp.
pub struct ToastQueue {
    pub queue: NotificationQueue,
}

impl ToastQueue {
    pub fn new() -> Self {
        let config = QueueConfig::new()
            .max_visible(3)
            .max_queued(10)
            .default_duration(Duration::from_secs(3))
            .position(ToastPosition::BottomRight);
        Self {
            queue: NotificationQueue::new(config),
        }
    }

    /// Success toast — gesture landed (3s auto-dismiss).
    pub fn push_success(&mut self, message: &str) {
        let toast = Toast::new(message)
            .icon(ToastIcon::Success)
            .style_variant(ToastStyle::Success)
            .duration(Duration::from_secs(3))
            .position(ToastPosition::BottomRight);
        self.queue.notify(toast);
    }

    /// Info toast — neutral feedback (3s auto-dismiss).
    pub fn push_info(&mut self, message: &str) {
        let toast = Toast::new(message)
            .icon(ToastIcon::Info)
            .style_variant(ToastStyle::Info)
            .duration(Duration::from_secs(3))
            .position(ToastPosition::BottomRight);
        self.queue.notify(toast);
    }

    /// Warning toast — something unexpected (5s auto-dismiss).
    pub fn push_warning(&mut self, message: &str) {
        let toast = Toast::new(message)
            .icon(ToastIcon::Warning)
            .style_variant(ToastStyle::Warning)
            .duration(Duration::from_secs(5))
            .position(ToastPosition::BottomRight);
        self.queue.notify(toast);
    }

    /// Undo toast — shows Redo action button (5s auto-dismiss).
    pub fn push_undo(&mut self, message: &str) {
        let toast = Toast::new(message)
            .icon(ToastIcon::Info)
            .style_variant(ToastStyle::Info)
            .action(ToastAction::new("Redo", "redo"))
            .duration(Duration::from_secs(5))
            .position(ToastPosition::BottomRight);
        self.queue.notify(toast);
    }

    /// Tick the queue — call from Msg::Tick when toasts are active.
    /// Returns true if any toast events need processing.
    pub fn tick(&mut self, delta: Duration) -> Vec<ftui::widgets::QueueAction> {
        self.queue.tick(delta)
    }

    /// Whether any toasts are visible or queued.
    pub fn is_active(&self) -> bool {
        !self.queue.is_empty()
    }

    /// Render the toast stack over the given area.
    pub fn render(&self, area: &Rect, frame: &mut Frame<'_>) {
        if self.queue.visible().is_empty() {
            return;
        }
        NotificationStack::new(&self.queue).render(*area, frame);
    }
}
