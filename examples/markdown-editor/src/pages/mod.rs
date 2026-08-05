//! Routed pages for the Markdown editor.
//!
//! Each feature directory owns one route-level page and its co-located child
//! components. Shared presentation helpers remain internal to the page layer.
//!
//! # Modules
//!
//! - [`explorer`] — Current-directory file explorer page.
//! - [`home`] — Landing page and recent-file actions.
//! - [`not_found`] — Unmatched-route fallback page.
//! - [`shared`] — Presentation helpers shared by routed pages.
//! - [`viewer`] — Markdown viewer page and viewer-route encoding.

mod explorer;
mod home;
mod not_found;
mod shared;
mod viewer;

pub(crate) use explorer::ExplorerPage;
pub(crate) use home::HomePage;
pub(crate) use not_found::NotFoundPage;
pub(crate) use viewer::{ViewerPage, viewer_location};
