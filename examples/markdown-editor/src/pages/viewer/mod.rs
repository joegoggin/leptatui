//! Markdown viewer feature.
//!
//! # Modules
//!
//! - [`components`] — Markdown document presentation.
//! - [`page`] — Viewer route-level component and route synchronization.
//! - [`style`] — Viewer page stylesheet registration.

mod components;
mod page;
mod style;

pub(crate) use page::{ViewerPage, viewer_location};
