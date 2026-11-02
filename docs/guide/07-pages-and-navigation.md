# 7. Pages and navigation

Prerequisites: [Chapter 6: Styling and layout](06-styling-and-layout.md).

Give the notes app an editor page and a preview page. Routing selects a view
using an in-memory path such as `/edit`; it does not start a web server.

Keep the draft provider and quit handler in `NotesApp`. Replace its view with:

```rust
view! {
    <Router initial_path="/edit">
        <Div class="notes-shell">
            <NoteHeading text="My notes" />
            <Div class="actions">
                <A href="/edit">"Edit"</A>
                <A href="/preview">"Preview"</A>
            </Div>
            <Routes fallback=NotFound>
                <Route path="/edit" view=NoteEditor />
                <Route path="/preview" view=NotePreview />
            </Routes>
        </Div>
    </Router>
}
```

`A` is an internal route anchor. `Link` opens a URL or local file with the
system handler. The shell outside `Routes` stays mounted, so its context-owned
draft survives navigation. The [complete routed checkpoint](../checkpoints/07-routed-notes.md)
includes `NoteEditor`, `NotePreview`, and `NotFound`; replace `src/main.rs`
with it to run this chapter.

Tab to Preview and activate it. It should show the latest draft; returning to
Edit should preserve the text. An active route anchor matches `A:active`.
Use `exact=true` when a root anchor should match only its exact destination.

## Programmatic navigation and history

Inside a component below `Router`, call `use_navigate()` to get a navigation
function. For example, a button can call
`navigate("/preview", NavigateOptions::default())` and return
`AppControl::Continue`. `use_location()` gives reactive pathname and search
state. `use_history()` exposes back and forward operations; history is local
to the running process. The [routing reference](../reference/routing.md)
contains a full navigation example.

## Parameters, queries, and nested pages

A path such as `/notes/:id` captures a parameter. A terminal `*wildcard`
captures a remaining path. Query values follow `?`, as in
`/preview?line=3`. Derive `RouteParams` and `QueryParams` for typed models,
then read them inside matched pages using `use_params` and `use_query`.
Required fields parse through `FromStr`; optional fields use `Option<T>`.
`#[param(name = "page-number")]` changes the external field name.

Use `with_query("/preview", &query_model)` to encode query values, rather than
concatenating raw strings. Derived query models also expose `to_query_string`.
Invalid required values produce an error; chapter 8 shows how to propagate it.
Parameter or query changes remount matched content.

For nested navigation, declare `ParentRoute` with child routes and render
`<Outlet />` in its layout component. This retains the parent layout while
switching child pages. See the [nested route example](../../crates/leptatui/tests/route_outlet.rs)
and the [routing reference](../reference/routing.md).

Run `cargo run --example multi_page_demo` for a separate app with shared
counter and theme state.

[Previous: Styling and layout](06-styling-and-layout.md) · [Documentation index](../README.md) · [Next: Handling failures](08-handling-failures.md)
