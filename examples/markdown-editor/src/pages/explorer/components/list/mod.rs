//! Explorer list and its co-located stylesheet.
//!
//! # Modules
//!
//! - [`component`] — Explorer rows, empty state, and recoverable errors.
//! - [`style`] — Explorer list stylesheet registration.

mod component;
mod style;

pub(in crate::pages::explorer) use component::{ExplorerList, ExplorerListProps};
