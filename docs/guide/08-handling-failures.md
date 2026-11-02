# 8. Handling failures

Prerequisites: [Chapter 7: Pages and navigation](07-pages-and-navigation.md).

Prepare for operations that can fail. A component returning
`ViewResult<impl IntoView>` can use `?` during setup. The component macro wraps
its final bare view expression in `Ok` for you.

This standalone page illustrates typed route/query conversion. Add it to a
`/notes/:id` route to try paths such as `/notes/1?line=3`:

```rust
#[derive(RouteParams)]
struct NoteParams {
    id: usize,
}

#[derive(QueryParams)]
struct NoteQuery {
    line: Option<usize>,
}

#[component]
fn NoteDetails() -> ViewResult<impl IntoView> {
    let params = use_params::<NoteParams>()?;
    let query = use_query::<NoteQuery>()?;
    view! { <Text>{format!("Note {}, line {:?}", params.id, query.line)}</Text> }
}
```

A malformed number, such as `/notes/abc`, raises the standard error screen.
The screen replaces the active UI and offers Back and Quit controls. Use
Esc or `b` to return through router history, or `q` to quit. Buttons also
support the usual focus and activation keys.

## Add context to an error

`view_error!("Could not load the note")` returns an intentional diagnostic.
`view_error!(error => "Could not load the note")` preserves an underlying
error as its source. Use it inside a fallible component when a lower-level
message needs application context:

```rust
#[component]
fn RequiredFile() -> ViewResult<impl IntoView> {
    let source = match std::fs::read_to_string("note.md") {
        Ok(source) => source,
        Err(error) => view_error!(error => "Could not load note.md"),
    };
    view! { <Paragraph>{source}</Paragraph> }
}
```

This small example performs a synchronous read during setup. For interactive
loading and saving, chapter 10 uses asynchronous filesystem operations.

Resource and action errors remain ordinary `Result` values. Render those
locally when a failed request should leave the editor usable. Use fallible
components when the view cannot be constructed at all. The runtime restores
the terminal before forwarding panic diagnostics to Rust's existing hook.

Run `cargo run --example error_handling` to try errors, navigation recovery,
and panic cleanup. The tutorial's async checkpoint in chapter 9 will display
request failures beside the draft instead of replacing the whole screen.

[Previous: Pages and navigation](07-pages-and-navigation.md) · [Documentation index](../README.md) · [Next: Async data](09-async-data.md)
