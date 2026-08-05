//! Root layout component and its co-located stylesheet.
//!
//! # Modules
//!
//! - [`component`] — Root frame and child-content boundary.
//! - [`style`] — Root layout stylesheet registration.

mod component;
mod style;

pub(crate) use component::{RootLayout, RootLayoutProps};
