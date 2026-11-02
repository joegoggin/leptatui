//! Fail fixture for an unsupported typed parameter attribute.

use leptatui::prelude::*;

/// Triggers parameter-attribute validation.
#[derive(QueryParams)]
struct InvalidAttributeQuery {
    /// Field using the unsupported `rename` key.
    #[param(rename = "page-number")]
    page: usize,
}

/// Provides the binary entry point required by trybuild.
fn main() {}
