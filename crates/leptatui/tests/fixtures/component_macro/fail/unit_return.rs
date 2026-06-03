//! Fail fixture for component functions returning unit.
//!
//! This binary triggers the diagnostic for annotated functions that explicitly
//! return `()`.

use leptatui::prelude::*;

/// Defines an unsupported component function returning unit.
#[component]
fn UnitReturn() -> () {}

/// Provides the binary entry point required by trybuild.
fn main() {}
