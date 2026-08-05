//! Explorer entry row and its co-located stylesheet.
//!
//! # Modules
//!
//! - [`component`] — Selected and unselected explorer entry presentation.
//! - [`style`] — Explorer entry-row stylesheet registration.

mod component;
mod style;

pub(in crate::pages::explorer) use component::{ExplorerEntryRow, ExplorerEntryRowProps};
