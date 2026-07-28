//! Workspace file explorer feature.
//!
//! # Modules
//!
//! - [`components`] — Explorer content and listing components.
//! - [`page`] — Explorer route-level component and keyboard behavior.

mod components;
mod page;

pub(crate) use page::{ExplorerPage, ExplorerPageProps};
