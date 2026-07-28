//! Routed user interface for the Markdown editor.
//!
//! The application shell owns route registration and global styling while each
//! routed page owns its controls and page-specific child components.
//!
//! # Modules
//!
//! - [`app`] — Application shell, routing, and global styling.
//! - [`explorer`] — Workspace file explorer page.
//! - [`home`] — Landing page and recent-file actions.
//! - [`not_found`] — Unmatched-route fallback page.
//! - [`shared`] — Presentation helpers shared by multiple pages.
//! - [`viewer`] — Markdown viewer page and viewer-route encoding.

mod app;
mod explorer;
mod home;
mod not_found;
mod shared;
mod viewer;

#[cfg(test)]
pub(crate) use app::app_view;
pub(crate) use app::app_view_at_path;
pub(crate) use viewer::viewer_location;
