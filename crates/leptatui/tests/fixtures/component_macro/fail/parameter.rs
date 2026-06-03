//! Fail fixture for component functions with parameters.
//!
//! This binary triggers the diagnostic for annotated functions that accept
//! parameters before props are supported.

use leptatui::prelude::*;

/// Defines an unsupported component function with a parameter.
///
/// # Arguments
///
/// * `label` — Text that would be rendered if parameters were supported.
#[component]
fn WithProps(label: String) -> leptatui::Node {
    leptatui::text(label)
}

/// Provides the binary entry point required by trybuild.
fn main() {}
