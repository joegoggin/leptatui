# 9. Async data

Prerequisites: [Chapter 8: Handling failures](08-handling-failures.md).

Add a mock service without requiring a network, account, or backend. Continue
from chapter 8 and add a data toolbar to the routed editor shell.

## Read with a resource

A resource runs an async fetcher when its reactive source changes or when you
call `refetch`. The toolbar creates one during component setup:

```rust
let loaded = Resource::new(
    || (),
    |_| async {
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        Ok::<_, String>(String::from("A note from the mock service"))
    },
);
```

`get()` returns `None` before completion, then `Some(output)`. The output can
itself be `Ok(value)` or `Err(error)`. `is_loading()` is separate from that
value: reloading retains the previous result. Older requests cannot overwrite
the result of a newer request.

The toolbar deliberately requires **Use loaded note** before replacing the
draft. A background read should not unexpectedly erase text the user is typing.
**Reload** fetches again; it does not apply the result automatically.

## Write with an action

An action starts only when you dispatch an input. Clone borrowed input before
moving it into its future:

```rust
let save = Action::new(|body: &String| {
    let body = body.clone();
    async move {
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        if body.trim().is_empty() {
            Err(String::from("Write something before saving"))
        } else {
            Ok(format!("Accepted {} bytes", body.len()))
        }
    }
});
```

`dispatch(body.get_untracked())` takes a snapshot. `is_pending()` reports work
in progress, `input()` is present while pending, `value()` retains the latest
output, and `version()` records completions. `clear()` clears action state.
Actions also ignore stale completions. If you want to refresh a resource after
a successful mutation, call `refetch()` or update the signal used as its source.

The mock save acknowledges the text but does not persist it. The next chapter
replaces it with real file operations. Start the app, wait for loading to finish
without pressing a key, apply the loaded note, then save. Clear the body and
save again to see a local error while the editor stays usable.

For work outside resources/actions, `spawn` schedules a `Send` future and
`spawn_local` supports local futures. Create async work under the running Tokio
app. `spawn_local` requires the managed app's local task set (or an explicit
Tokio local task set). Prefer resource/action handles when the UI needs pending and result
state. See the [async reference](../reference/async.md) and run
`cargo run --example async_crud` for refetch-after-mutation behavior.

## Connect the toolbar

Add this component above `NotesApp`. It combines the resource and action above,
clones handles for callbacks, and renders pending, success, and error states:

```rust
#[component]
fn DataToolbar() -> impl IntoView {
    let Draft { body, status, .. } = expect_context::<Draft>();
    let loaded = Resource::new(
        || (),
        |_| async {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            Ok::<_, String>(String::from("A note from the mock service"))
        },
    );
    let save = Action::new(|body: &String| {
        let body = body.clone();
        async move {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            if body.trim().is_empty() {
                Err(String::from("Write something before saving"))
            } else {
                Ok(format!("Accepted {} bytes", body.len()))
            }
        }
    });
    let reload = loaded.clone();
    let apply = loaded.clone();
    let save_button = save.clone();
    view! {
        <Div>
            <Div class="notes-shell__actions">
                <Button on_press=move || {
                    reload.refetch();
                }>"Reload"</Button>
                <Button on_press=move || {
                    if let Some(Ok(value)) = apply.get_untracked() {
                        body.set(value);
                        status.set(String::from("Loaded note applied"));
                    }
                }>"Use loaded note"</Button>
                <Button on_press=move || {
                    save_button.dispatch(body.get_untracked());
                }>"Save body"</Button>
            </Div>
            <Text>{move || {
                if loaded.is_loading() {
                    String::from("Loading...")
                } else {
                    match loaded.get() {
                        None => String::from("Not loaded"),
                        Some(Ok(_)) => String::from("Loaded; choose Use loaded note to apply"),
                        Some(Err(error)) => format!("Load failed: {error}"),
                    }
                }
            }}</Text>
            <Text>{move || {
                if save.is_pending() {
                    String::from("Saving...")
                } else {
                    match save.value().get() {
                        None => String::from("No save yet"),
                        Some(Ok(message)) => message,
                        Some(Err(error)) => format!("Save failed: {error}"),
                    }
                }
            }}</Text>
        </Div>
    }
}
```

In `NotesApp`, insert `<DataToolbar />` immediately before
`<Routes fallback=NotFound>`. Keep the routes and shared draft from chapter 8.

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
fn DataToolbar() -> impl IntoView {
    let Draft { body, status, .. } = expect_context::<Draft>();
    let loaded = Resource::new(
        || (),
        |_| async {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            Ok::<_, String>(String::from("A note from the mock service"))
        },
    );
    let save = Action::new(|body: &String| {
        let body = body.clone();
        async move {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            if body.trim().is_empty() {
                Err(String::from("Write something before saving"))
            } else {
                Ok(format!("Accepted {} bytes", body.len()))
            }
        }
    });
    let reload = loaded.clone();
    let apply = loaded.clone();
    let save_button = save.clone();
    view! {
        <Div>
            <Div class="notes-shell__actions">
                <Button on_press=move || {
                    reload.refetch();
                }>"Reload"</Button>
                <Button on_press=move || {
                    if let Some(Ok(value)) = apply.get_untracked() {
                        body.set(value);
                        status.set(String::from("Loaded note applied"));
                    }
                }>"Use loaded note"</Button>
                <Button on_press=move || {
                    save_button.dispatch(body.get_untracked());
                }>"Save body"</Button>
            </Div>
            <Text>{move || {
                if loaded.is_loading() {
                    String::from("Loading...")
                } else {
                    match loaded.get() {
                        None => String::from("Not loaded"),
                        Some(Ok(_)) => String::from("Loaded; choose Use loaded note to apply"),
                        Some(Err(error)) => format!("Load failed: {error}"),
                    }
                }
            }}</Text>
            <Text>{move || {
                if save.is_pending() {
                    String::from("Saving...")
                } else {
                    match save.value().get() {
                        None => String::from("No save yet"),
                        Some(Ok(message)) => message,
                        Some(Err(error)) => format!("Save failed: {error}"),
                    }
                }
            }}</Text>
        </Div>
    }
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
                <DataToolbar />
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

[Previous: Handling failures](08-handling-failures.md) · [Documentation index](../README.md) · [Next: Files and editors](10-files-and-editors.md)
