//! Pass fixture for typed route and query parameter models.

use leptatui::prelude::*;

/// Supported route parameter model.
#[derive(RouteParams)]
struct ViewerParams {
    /// Required decoded path.
    path: String,
    /// Optional parsed line number.
    line: Option<usize>,
}

/// Supported query parameter model with one renamed field.
#[derive(QueryParams)]
struct ViewerQuery {
    /// Required page number read from a hyphenated query name.
    #[param(name = "page-number")]
    page: u16,
}

/// Requires the generated trait implementations at compile time.
fn main() {
    fn route_model<T: leptatui::RouteParams>() {}
    fn query_model<T: leptatui::QueryParams>() {}

    route_model::<ViewerParams>();
    query_model::<ViewerQuery>();
    let query = ViewerQuery { page: 4 };
    assert_eq!(query.to_query_string(), "page-number=4");
}
