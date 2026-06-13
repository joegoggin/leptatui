//! Fail fixture for component functions with unsupported prop patterns.
//!
//! This binary triggers the diagnostic for annotated functions that use
//! destructuring parameters as props.

use leptatui::prelude::*;

/// Defines an unsupported component function with a destructuring prop.
///
#[component]
fn WithProps((label,): (String,)) -> leptatui::Node {
    leptatui::text(label)
}

/// Provides the binary entry point required by trybuild.
fn main() {}
