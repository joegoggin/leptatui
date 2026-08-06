//! Active notification overlay rendering.

use leptatui::prelude::*;

use super::{
    super::{Notification, NotificationLevel, use_notifications},
    style::use_notifications_styles,
};

/// Renders active notifications above routed page content.
///
/// # Returns
///
/// A dynamic notification list.
#[component]
pub(crate) fn Notifications() -> impl IntoView {
    let notifications = use_notifications();

    use_notifications_styles();

    dynamic(move || {
        let rows = notifications.notifications.get();
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
        NotificationLevel::Success => "notifications__item notifications__item--success",
        NotificationLevel::Error => "notifications__item notifications__item--error",
        NotificationLevel::Info => "notifications__item notifications__item--info",
        NotificationLevel::Warning => "notifications__item notifications__item--warning",
    };

    view! {
        <Div class={class}>
            <Text>{format!("{}: {}", notification.title, notification.message)}</Text>
        </Div>
    }
    .into_view()
}
