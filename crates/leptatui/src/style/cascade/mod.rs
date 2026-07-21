//! Stylesheet declarations, selectors, media queries, and cascade resolution.
//!
//! # Modules
//!
//! - [`declarations`] - Optional style properties authored by rules.
//! - [`media`] - Terminal viewport constraints.
//! - [`selector`] - View selector matching.
//! - [`stylesheet`] - Rule storage and cascade resolution.

mod declarations;
mod media;
mod selector;
mod stylesheet;

pub use declarations::StyleDeclarations;
pub use media::{MediaQuery, ViewportSize};
pub use selector::StyleSelector;
pub use stylesheet::{StyleRule, Stylesheet};
