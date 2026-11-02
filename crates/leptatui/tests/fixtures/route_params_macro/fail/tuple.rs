//! Fail fixture for a tuple query parameter model.

use leptatui::prelude::*;

/// Triggers the named-field model requirement.
#[derive(QueryParams)]
struct TupleQuery(String);

/// Provides the binary entry point required by trybuild.
fn main() {}
