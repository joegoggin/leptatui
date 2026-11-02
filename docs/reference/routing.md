# Routing reference

Start with [Pages and navigation](../guide/07-pages-and-navigation.md).

Multi-page apps use URL-like locations and declarative route components. Wrap
the application shell in `Router`, place route declarations inside `Routes`,
and navigate with internal `A` anchors or `use_navigate()`. Content outside
`Routes` remains mounted, and `ParentRoute` plus `Outlet` provides retained
nested layouts. An `A` whose destination matches the current route matches the
`:active` stylesheet pseudo-class. Route matching no longer injects an
`active` class, so authored route-state rules should use `A:active` or nested
`&:active` selectors.

Use props for required parent-to-child inputs and local component
configuration. Use context for app-wide state that many routes or deeply nested
descendants need to read or update, such as theme variables or persisted
settings. Keep shared state owned by the root when navigation
should not reset it; add explicit reset behavior when a route change should
clear shared state. Descendant pages can read required context with
`expect_context()` or optional context with `use_context()`.

```rust
use leptatui::prelude::*;

#[component]
fn Nav() -> impl IntoView {
    view! {
        <Div style={TuiStyle::new().display(Display::Flex)}>
            <A href="/" exact=true>"Home"</A>
            <A href="/counter">"Counter"</A>
            <A href="/settings">"Settings"</A>
        </Div>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    let counter = expect_context::<RwSignal<i32>>();

    view! {
        <Div>
            <Text>"Home"</Text>
            {move || {
                view! {
                    <Text>{format!("Count: {}", counter.get())}</Text>
                }
            }}
        </Div>
    }
}

#[component]
fn CounterPage() -> impl IntoView {
    let counter = expect_context::<RwSignal<i32>>();

    view! {
        <Div>
            <Text>"Counter"</Text>
            <Button on_press=move || {
                counter.update(|count| *count += 1);
                AppControl::Continue
            }>
                "Increment"
            </Button>
        </Div>
    }
}

#[component]
fn SettingsPage() -> impl IntoView {
    let location = use_location();

    view! {
        <Text>{move || format!("Current path: {}", location.pathname().get())}</Text>
    }
}

#[component]
fn NotFoundPage() -> impl IntoView {
    view! { <Text>"Page not found"</Text> }
}

#[component]
fn Root() -> impl IntoView {
    let counter = RwSignal::new(0);

    provide_context(counter);

    view! {
        <Router initial_path="/">
            <Div>
                <Nav />
                <Routes fallback=NotFoundPage>
                    <Route path="/" view=HomePage />
                    <Route path="/counter" view=CounterPage />
                    <Route path="/settings" view=SettingsPage />
                </Routes>
            </Div>
        </Router>
    }
}

#[tokio::main]
async fn main() -> leptatui::app::Result<()> {
    let view = view! { <Root /> };
    App::new(view).run().await
}
```

Route patterns accept static segments, `:parameter` segments, and a terminal
`*wildcard`. Matched pages read decoded path and query values through derived
models:

```rust
use leptatui::prelude::*;

#[derive(RouteParams)]
struct ViewerParams {
    path: String,
}

#[derive(QueryParams)]
struct ViewerQuery {
    line: Option<usize>,
}

#[component]
fn ViewerPage() -> ViewResult<impl IntoView> {
    let params = use_params::<ViewerParams>()?;
    let query = use_query::<ViewerQuery>()?;

    view! {
        <Text>{format!("{} at {:?}", params.path, query.line)}</Text>
    }
}
```

Required fields implement `FromStr`; missing or malformed values propagate
through `?` to the standard error screen. Optional fields use `Option<T>`, and
`#[param(name = "page-number")]` maps a Rust field to a different parameter
name. Derived query models also serialize their `Display` values with
`to_query_string()`. Use `with_query("/view", &query)` to construct a location
with percent-encoded keys and values; absent optional fields are omitted.
Parameter or query changes remount matched route content while shared UI outside
`<Routes>` remains mounted. Read the current raw location with `use_location()`.
Programmatic navigation uses `use_navigate()`, while `use_history()` exposes
process-local back and forward controls.

See `crates/leptatui/examples/multi_page_demo.rs` or run
`cargo run --example multi_page_demo` for a complete routing and context
example with shared counter and theme state.

[Documentation index](../README.md) · [Feature index](features.md)
