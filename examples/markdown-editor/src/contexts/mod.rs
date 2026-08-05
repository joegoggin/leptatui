//! Application-level context providers.
//!
//! # Modules
//!
//! - [`notification`] — Shared user notification state and rendering.

mod notification;

pub(crate) use notification::{Notifications, provide_notification_context, use_notifications};
