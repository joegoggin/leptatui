/// Shared root-owned state exposed to route page branches.
#[derive(Clone, Copy)]
struct MacroSharedCount(ReadSignal<usize>);

/// Typed route parameters used by the matched-page remount fixture.
#[derive(RouteParams)]
struct MacroTypedRouteParams {
    /// Parsed numeric identifier captured from a renamed route parameter.
    #[param(name = "item-id")]
    item_id: usize,
}

/// Typed query parameters used by the matched-page remount fixture.
#[derive(QueryParams)]
struct MacroTypedQueryParams {
    /// Required decoded query label.
    label: String,
    /// Optional parsed page number.
    page: Option<usize>,
}
