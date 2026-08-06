//! Fail fixture for duplicate typed parameter mappings.

use leptatui::prelude::*;

/// Triggers duplicate mapping validation.
#[derive(RouteParams)]
struct DuplicateParams {
    /// First mapping for `value`.
    value: String,
    /// Duplicate mapping for `value`.
    #[param(name = "value")]
    alias: String,
}

/// Provides the binary entry point required by trybuild.
fn main() {}
