//! Viewer document component and its co-located stylesheet.
//!
//! # Modules
//!
//! - [`component`] — Declarative Markdown content and editor diagnostics.
//! - [`style`] — Viewer document stylesheet registration.

mod component;
mod style;

pub(in crate::pages::viewer) use component::{ViewerDocument, ViewerDocumentProps};
