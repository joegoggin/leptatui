//! Explorer page content and its co-located stylesheet.
//!
//! # Modules
//!
//! - [`component`] — Explorer content rendered from page-owned signals.
//! - [`style`] — Explorer content stylesheet registration.

mod component;
mod style;

pub(in crate::pages::explorer) use component::{ExplorerContent, ExplorerContentProps};
