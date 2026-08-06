//! Fail fixture for a generic typed parameter model.

use leptatui::prelude::*;

/// Triggers the non-generic model requirement.
#[derive(RouteParams)]
struct GenericParams<T> {
    /// Generic parameter value.
    value: T,
}

/// Provides the binary entry point required by trybuild.
fn main() {}
