//! Markdown viewer feature.
//!
//! # Modules
//!
//! - [`components`] — Markdown document presentation.
//! - [`page`] — Viewer route-level component and route synchronization.

mod components;
mod page;

pub(crate) use page::{ViewerPage, ViewerPageProps, viewer_location};
