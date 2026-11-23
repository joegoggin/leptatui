# 10. Files and editors

Prerequisites: [Chapter 9: Async data](09-async-data.md).

Replace the mock service with real note files. Keep the editor and routes
from chapter 9, and replace the data toolbar with load/save controls.
It stores the body as `notes-data/note.md`, relative to the notes project's
working directory. The title remains in memory.

## Scope file operations to a directory

`use_file_system` initializes a canonical directory boundary and can fail, so
the toolbar returns `ViewResult<impl IntoView>`. Use options to create a missing
root deliberately:

```rust
let files = use_file_system_with_options(
    "notes-data",
    FileSystemOptions::new().create_root(true),
)?;
let latest_read = RwSignal::new(None::<FileOperation<String>>);
let latest_write = RwSignal::new(None::<FileOperation<()>>);
```

Each operation starts immediately and returns a reactive handle. Keep it to
show loading and results:

```rust
latest_read.set(Some(files.read_file_as_string("note.md")));
latest_write.set(Some(files.write_and_replace_file("note.md", "# My note\n")));
```

A `FileOperation<T>` is an `Action<(), std::io::Result<T>>`. Observe pending,
value, and version state; `dispatch(())` retries the same captured arguments.
Retain separate handles if both read and write status need to stay visible.

Run the app, edit the body, activate **Save body**, then **Load body**.
Choose **Use loaded note** to apply the read result. Restart the app and load
again to verify persistence. Loading before saving displays a missing-file
error locally; it does not erase the draft.

Relative paths resolve inside the root. Absolute paths and symlink targets
must remain inside it. Leading `~` expands to the user's home and still obeys
the boundary. Recursive root deletion is prohibited. The
[filesystem reference](../reference/files-and-editors.md) and
`cargo run --example file_system_showcase` cover path resolution, metadata,
directory listings, binary/text reads, directory creation, writes, append,
atomic replacement, copy, rename, deletion, retries, and expected failures.

## Connect file loading and saving

Replace the entire `DataToolbar` definition with this `FileToolbar`. Replace
`<DataToolbar />` in `NotesApp` with `<FileToolbar />`:

```rust
#[component]
fn FileToolbar() -> ViewResult<impl IntoView> {
    let Draft { body, status, .. } = expect_context::<Draft>();
    let files =
        use_file_system_with_options("notes-data", FileSystemOptions::new().create_root(true))?;
    let latest_read = RwSignal::new(None::<FileOperation<String>>);
    let latest_write = RwSignal::new(None::<FileOperation<()>>);
    let read_files = files.clone();
    view! {
        <Div>
            <Div class="notes-shell__actions">
                <Button on_press=move || {
                    latest_read.set(Some(read_files.read_file_as_string("note.md")));
                }>"Load body"</Button>
                <Button on_press=move || {
                    if let Some(read) = latest_read.get_untracked() {
                        read.value().with_untracked(|result| {
                            if let Some(Ok(value)) = result {
                                body.set(value.clone());
                                status.set(String::from("File applied to draft"));
                            }
                        });
                    }
                }>"Use loaded note"</Button>
                <Button on_press=move || {
                    latest_write.set(Some(files.write_and_replace_file(
                        "note.md", body.get_untracked(),
                    )));
                }>"Save body"</Button>
            </Div>
            <Text>{move || match latest_read.get() {
                None => String::from("No read yet"),
                Some(read) if read.is_pending() => String::from("Reading..."),
                Some(read) => read.value().with(|result| match result {
                    Some(Ok(_)) => String::from("Read complete; choose Use loaded note"),
                    Some(Err(error)) => format!("Read failed: {error}"),
                    None => String::from("Waiting for read"),
                }),
            }}</Text>
            <Text>{move || match latest_write.get() {
                None => String::from("No save yet"),
                Some(write) if write.is_pending() => String::from("Saving..."),
                Some(write) => write.value().with(|result| match result {
                    Some(Ok(())) => String::from("Saved notes-data/note.md"),
                    Some(Err(error)) => format!("Save failed: {error}"),
                    None => String::from("Waiting for save"),
                }),
            }}</Text>
        </Div>
    }
}
```

## Optional: Choose a file

The selector owns its own full-screen interface. To try it, add this component
above `NotesApp`, then insert `<FilePicker />` after `<FileToolbar />` in the
shell. The selected path is displayed without replacing the draft:

```rust
#[component]
fn FilePicker() -> impl IntoView {
    let selector = use_file_selector();
    let open = selector.clone();
    view! {
        <Div>
            <Button on_press=move || {
                open.select_with_options(
                    FileSelectorOptions::new()
                        .starting_directory("notes-data")
                        .extensions(["md", "markdown"]),
                );
            }>"Choose Markdown file"</Button>
            <Text>{move || match selector.get_file() {
                Some(path) => format!("Selected: {}", path.display()),
                None => String::from("No file selected"),
            }}</Text>
        </Div>
    }
}
```

Read `selector.get_file()` inside a tracked closure or effect to observe a
successful canonical path selection. `select()` uses the current directory
without extension filtering; `clear()` removes the saved selection. Opening or
canceling preserves the previous selection. Enter selects, Esc or Ctrl+C
cancels, and `q` exits the application. A picked path is not automatically
inside your filesystem root: use an appropriate root before operating on it.

## Optional: Open an external editor

Add this component above `NotesApp` and insert `<EditorTools />` after
`<FileToolbar />` in the shell. Save a note first before trying the saved-file
button. The first button edits the draft signal through a temporary Markdown
file; the second edits the saved file:

```rust
#[component]
fn EditorTools() -> impl IntoView {
    let Draft { body, .. } = expect_context::<Draft>();
    let editor = use_editor();
    let text_editor = editor.clone();
    let file_editor = editor.clone();
    view! {
        <Div>
            <Button on_press=move || {
                text_editor.edit_text(body);
            }>"Edit draft externally"</Button>
            <Button on_press=move || {
                file_editor.edit_file("notes-data/note.md");
            }>"Edit saved file externally"</Button>
            <Text>{move || match editor.status() {
                None => String::from("Editor idle"),
                Some(EditorStatus::Pending) => String::from("Editor open"),
                Some(EditorStatus::Complete) => String::from("Editor complete"),
                Some(EditorStatus::Error(error)) => format!("Editor failed: {error}"),
            }}</Text>
        </Div>
    }
}
```

The editor is chosen from `VISUAL`, then `EDITOR`, then `vi`. For example,
set `VISUAL="nvim -f"` before launching your app.

The runtime restores the terminal, waits for the editor, and resumes the same
mounted UI. Observe `editor.status()` for `Pending`, `Complete`, or `Error` and
call `clear()` to clear a completed status. A failed text edit preserves the
signal. A successful file edit does not automatically update your draft;
load the file again explicitly.

For another synchronous terminal integration, call
`use_app_handle().suspend_terminal(move || { /* synchronous work */ })` from
an event callback after obtaining the handle during setup. Work runs on the
app thread while the terminal is restored; do not run it directly during
normal rendering. Run `cargo run --example external_editor` or explore the
[Markdown editor application](../../examples/markdown-editor/README.md).

The selector and external editor sections are optional integrations. Remove
`FilePicker`, `EditorTools`, and their shell tags after trying them. The final
application below uses the fixed `notes-data/note.md` path and is the version
to continue with in chapter 11. No additional Cargo dependencies are required.

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
fn FileToolbar() -> ViewResult<impl IntoView> {
    let Draft { body, status, .. } = expect_context::<Draft>();
    let files =
        use_file_system_with_options("notes-data", FileSystemOptions::new().create_root(true))?;
    let latest_read = RwSignal::new(None::<FileOperation<String>>);
    let latest_write = RwSignal::new(None::<FileOperation<()>>);
    let read_files = files.clone();
    view! {
        <Div>
            <Div class="notes-shell__actions">
                <Button on_press=move || {
                    latest_read.set(Some(read_files.read_file_as_string("note.md")));
                }>"Load body"</Button>
                <Button on_press=move || {
                    if let Some(read) = latest_read.get_untracked() {
                        read.value().with_untracked(|result| {
                            if let Some(Ok(value)) = result {
                                body.set(value.clone());
                                status.set(String::from("File applied to draft"));
                            }
                        });
                    }
                }>"Use loaded note"</Button>
                <Button on_press=move || {
                    latest_write.set(Some(files.write_and_replace_file(
                        "note.md", body.get_untracked(),
                    )));
                }>"Save body"</Button>
            </Div>
            <Text>{move || match latest_read.get() {
                None => String::from("No read yet"),
                Some(read) if read.is_pending() => String::from("Reading..."),
                Some(read) => read.value().with(|result| match result {
                    Some(Ok(_)) => String::from("Read complete; choose Use loaded note"),
                    Some(Err(error)) => format!("Read failed: {error}"),
                    None => String::from("Waiting for read"),
                }),
            }}</Text>
            <Text>{move || match latest_write.get() {
                None => String::from("No save yet"),
                Some(write) if write.is_pending() => String::from("Saving..."),
                Some(write) => write.value().with(|result| match result {
                    Some(Ok(())) => String::from("Saved notes-data/note.md"),
                    Some(Err(error)) => format!("Save failed: {error}"),
                    None => String::from("Waiting for save"),
                }),
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
                <FileToolbar />
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

[Previous: Async data](09-async-data.md) · [Documentation index](../README.md) · [Next: Documents and media](11-documents-and-media.md)
