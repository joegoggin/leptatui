//! Shared notification state and terminal rendering.

use std::time::Duration;

use leptatui::prelude::*;

/// Visual severity assigned to a notification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NotificationLevel {
    /// Reports a successful operation.
    Success,
    /// Reports a failed operation.
    Error,
    /// Reports neutral information.
    Info,
    /// Reports a recoverable warning.
    Warning,
}

/// One notification rendered in the application shell.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Notification {
    /// Process-local notification identifier.
    id: u64,
    /// Short notification heading.
    title: String,
    /// Detailed notification message.
    message: String,
    /// Visual notification severity.
    level: NotificationLevel,
}

/// Shared notification queue and lifecycle operations.
#[derive(Clone, Copy, Debug)]
pub(crate) struct NotificationContext {
    /// Signal containing active notifications.
    notifications: RwSignal<Vec<Notification>>,
    /// Signal containing the next notification identifier.
    next_id: RwSignal<u64>,
}

impl NotificationContext {
    /// Time a notification remains visible before automatic dismissal.
    const DISPLAY_DURATION: Duration = Duration::from_secs(5);

    /// Creates an empty notification context.
    ///
    /// # Returns
    ///
    /// A [`NotificationContext`] with no active notifications.
    fn new() -> Self {
        Self {
            notifications: RwSignal::new(Vec::new()),
            next_id: RwSignal::new(0),
        }
    }

    /// Shows a success notification.
    ///
    /// # Arguments
    ///
    /// * `title` — Short notification heading.
    /// * `message` — Detailed notification message.
    pub(crate) fn show_success(&self, title: impl Into<String>, message: impl Into<String>) {
        self.show(title, message, NotificationLevel::Success);
    }

    /// Shows an error notification.
    ///
    /// # Arguments
    ///
    /// * `title` — Short notification heading.
    /// * `message` — Detailed notification message.
    pub(crate) fn show_error(&self, title: impl Into<String>, message: impl Into<String>) {
        self.show(title, message, NotificationLevel::Error);
    }

    /// Shows an informational notification.
    ///
    /// # Arguments
    ///
    /// * `title` — Short notification heading.
    /// * `message` — Detailed notification message.
    pub(crate) fn show_info(&self, title: impl Into<String>, message: impl Into<String>) {
        self.show(title, message, NotificationLevel::Info);
    }

    /// Shows a warning notification.
    ///
    /// # Arguments
    ///
    /// * `title` — Short notification heading.
    /// * `message` — Detailed notification message.
    pub(crate) fn show_warning(&self, title: impl Into<String>, message: impl Into<String>) {
        self.show(title, message, NotificationLevel::Warning);
    }

    /// Dismisses a notification by identifier.
    ///
    /// # Arguments
    ///
    /// * `id` — Identifier of the notification to remove.
    pub(crate) fn dismiss(&self, id: u64) {
        self.notifications
            .update(|items| items.retain(|item| item.id != id));
    }

    /// Shows a notification and schedules automatic dismissal.
    ///
    /// # Arguments
    ///
    /// * `title` — Short notification heading.
    /// * `message` — Detailed notification message.
    /// * `level` — Visual notification severity.
    fn show(&self, title: impl Into<String>, message: impl Into<String>, level: NotificationLevel) {
        let id = self.next_id.get_untracked();
        self.next_id.set(id.wrapping_add(1));
        self.notifications.update(|items| {
            items.push(Notification {
                id,
                title: title.into(),
                message: message.into(),
                level,
            });
        });

        if tokio::runtime::Handle::try_current().is_ok() {
            let notifications = *self;
            spawn(async move {
                tokio::time::sleep(Self::DISPLAY_DURATION).await;
                notifications.dismiss(id);
            });
        }
    }
}

/// Provides shared notification state to descendant components.
///
/// # Returns
///
/// A [`NotificationContext`] inserted into the current component context.
pub(crate) fn provide_notification_context() -> NotificationContext {
    let notifications = NotificationContext::new();
    provide_context(notifications);
    notifications
}

/// Returns the nearest shared notification context.
///
/// # Returns
///
/// A [`NotificationContext`] provided by the application shell.
///
/// # Panics
///
/// Panics if no notification context exists.
pub(crate) fn use_notifications() -> NotificationContext {
    expect_context::<NotificationContext>()
}

/// Renders active notifications above routed page content.
///
/// # Returns
///
/// A dynamic notification list.
#[component]
pub(crate) fn Notifications() -> impl IntoView {
    let notifications = use_notifications();

    dynamic(move || {
        let rows = notifications.notifications.get_untracked();
        div(rows
            .into_iter()
            .map(render_notification)
            .collect::<Vec<_>>())
        .with_classes("notifications")
    })
}

/// Renders one notification row.
///
/// # Arguments
///
/// * `notification` — Notification snapshot to render.
///
/// # Returns
///
/// An [`AnyView`] containing the notification title and message.
fn render_notification(notification: Notification) -> AnyView {
    let class = match notification.level {
        NotificationLevel::Success => "notification success",
        NotificationLevel::Error => "notification error",
        NotificationLevel::Info => "notification info",
        NotificationLevel::Warning => "notification warning",
    };

    view! {
        <Div class={class}>
            <Text>{format!("{}: {}", notification.title, notification.message)}</Text>
        </Div>
    }
    .into_view()
}

#[cfg(test)]
/// Unit tests for shared notification state.
mod tests {
    use leptatui::prelude::{GetUntracked, Owner};

    use super::*;

    /// Verifies notifications can be shown and dismissed.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// notifications.show_error("Save failed", "disk full")
    /// notifications.dismiss(0)
    /// ```
    ///
    /// # Assertions
    ///
    /// - Showing an error appends one notification.
    /// - The notification retains its title, message, and severity.
    /// - Dismissing its identifier removes it.
    #[test]
    fn notifications_are_added_and_dismissed() {
        let owner = Owner::new();
        let notifications = owner.with(NotificationContext::new);

        notifications.show_error("Save failed", "disk full");

        let items = notifications.notifications.get_untracked();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Save failed");
        assert_eq!(items[0].message, "disk full");
        assert_eq!(items[0].level, NotificationLevel::Error);

        notifications.dismiss(items[0].id);
        assert!(notifications.notifications.get_untracked().is_empty());
    }
}
