//! Fail fixture for component functions without return values.
//!
//! This binary triggers the diagnostic for annotated functions that omit an
//! explicit return type.

use leptatui::prelude::*;

/// Defines an unsupported component function with no return type.
#[component]
fn NoReturn() {}

/// Provides the binary entry point required by trybuild.
fn main() {}
