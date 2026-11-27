# 8. Handling failures

Prerequisites: [Chapter 7: Pages and navigation](07-pages-and-navigation.md).

Prepare for operations that can fail. A component returning
`ViewResult<impl IntoView>` can use `?` during setup. The component macro wraps
its final bare view expression in `Ok` for you. The parameter and query models
derive `Clone` because the tracked view captures their values.

This standalone page illustrates typed route/query conversion. Add it to a
`/notes/:id` route to try paths such as `/notes/1?line=3`:

```rust
#[derive(Clone, RouteParams)]
struct NoteParams {
    id: usize,
}

#[derive(Clone, QueryParams)]
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
and panic cleanup. Chapter 9 will display
request failures beside the draft instead of replacing the whole screen.

## Wire the demonstrations into the app

Add both components and the parameter/query models above `NotesApp`. In its
navigation `Div`, after the Preview link, add:

```rust
<A href="/notes/1?line=3">"Note details"</A>
<A href="/notes/abc">"Invalid note"</A>
<A href="/required-file">"Read note.md"</A>
```

Inside `Routes`, after the preview route, add:

```rust
<Route path="/notes/:id" view=NoteDetails />
<Route path="/required-file" view=RequiredFile />
```

Run `cargo run` and open **Note details**, then **Invalid note**. The latter
should display the error screen; press `b` to return to the previous page.
**Read note.md** reads a separate `note.md` in your working directory. If it
does not exist, this intentionally demonstrates an error with context. This
file is separate from the `notes-data/note.md` introduced in chapter 10.
Keep these routes in subsequent chapters.

## Complete code

Replace `src/main.rs` with the following. Use the Cargo manifest from
[chapter 1](01-getting-started.md) and run `cargo run` in your application
project. Ctrl+Q quits.

```rust
use leptatui::prelude::*;

#[derive(Clone, Copy)]
struct Draft {
    title: RwSignal<String>,
    body: RwSignal<String>,
    status: RwSignal<String>,
}

#[component]
fn NoteHeading(#[prop(into)] text: String) -> impl IntoView {
    view! { <H1>{text}</H1> }
}

#[component]
fn NoteEditor() -> impl IntoView {
    let Draft { title, body, status } = expect_context::<Draft>();

    view! {
        <Div>
            <Form
                on_submit=move || {
                    status.set(String::from("Submitted in memory"));
                }
                on_cancel=move || {
                    status.set(String::from("Canceled; draft kept"));
                }
            >
                <Text>"Title"</Text>
                <Input value=title placeholder="Title"
                    on_input=move || {
                        status.set(String::from("Editing"));
                    }
                />
                <Text>"Body"</Text>
                <TextArea value=body placeholder="Write a note"
                    on_input=move || {
                        status.set(String::from("Editing"));
                    }
                />
            </Form>
            <Text>{status}</Text>
            <Text>"Tab focuses. i edits. Esc returns to normal mode. Enter submits in normal mode. Ctrl+Q quits."</Text>
        </Div>
    }
}

#[component]
fn NotePreview() -> impl IntoView {
    let Draft { title, body, .. } = expect_context::<Draft>();
    view! {
        <Div>
            <H2>{title}</H2>
            <Paragraph>{body}</Paragraph>
        </Div>
    }
}

#[component]
fn NotFound() -> impl IntoView {
    view! { <Text>"Page not found. Choose Edit or Preview."</Text> }
}

#[derive(Clone, RouteParams)]
struct NoteParams {
    id: usize,
}

#[derive(Clone, QueryParams)]
struct NoteQuery {
    line: Option<usize>,
}

#[component]
fn NoteDetails() -> ViewResult<impl IntoView> {
    let params = use_params::<NoteParams>()?;
    let query = use_query::<NoteQuery>()?;
    view! { <Text>{format!("Note {}, line {:?}", params.id, query.line)}</Text> }
}

#[component]
fn RequiredFile() -> ViewResult<impl IntoView> {
    let source = match std::fs::read_to_string("note.md") {
        Ok(source) => source,
        Err(error) => view_error!(error => "Could not load note.md"),
    };
    view! { <Paragraph>{source}</Paragraph> }
}

#[component]
fn NotesApp() -> impl IntoView {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return KeyControl::Exit;
        }
        KeyControl::Pass
    });
    let title = RwSignal::new(String::from("First note"));
    let body = RwSignal::new(String::from("Learn to build a terminal application."));
    let status = RwSignal::new(String::from("Editing"));

    provide_context(Draft { title, body, status });

    stylesheet! {
        H1 { fg: Color::LightCyan; }
        A:active { fg: Color::LightGreen; }
        Button:focus { fg: Color::Yellow; }

        .notes-shell {
            padding: TuiSpacing::uniform(1);
            &__actions {
                display: Display::Flex;
                gap: Axes::all(Length::cells(1.0));
                @media (max-width: 60) {
                    flex_direction: FlexDirection::Column;
                }
            }
        }
    }
    view! {
        <Router initial_path="/edit">
            <Div class="notes-shell">
                <NoteHeading text="My notes" />
                <Div class="notes-shell__actions">
                    <A href="/edit">"Edit"</A>
                    <A href="/preview">"Preview"</A>
                    <A href="/notes/1?line=3">"Note details"</A>
                    <A href="/notes/abc">"Invalid note"</A>
                    <A href="/required-file">"Read note.md"</A>
                </Div>
                    <Routes fallback=NotFound>
                    <Route path="/edit" view=NoteEditor />
                    <Route path="/preview" view=NotePreview />
                    <Route path="/notes/:id" view=NoteDetails />
                    <Route path="/required-file" view=RequiredFile />
                </Routes>
            </Div>
        </Router>
    }
}

#[tokio::main]
async fn main() -> leptatui::app::Result<()> {
    App::new(view! { <NotesApp /> }).run().await
}
```

[Previous: Pages and navigation](07-pages-and-navigation.md) · [Documentation index](../README.md) · [Next: Async data](09-async-data.md)
