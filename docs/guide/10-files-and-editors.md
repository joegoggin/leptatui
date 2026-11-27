# 10. Files and editors

Prerequisites: [Chapter 9: Async data](09-async-data.md).

Replace the mock service with real note files. Delete
`src/layouts/root/components/data_toolbar.rs` and the `src/services` directory;
remove their module declarations and re-exports, including `mod services;`
in `main.rs`. Create `src/layouts/root/components/file_toolbar.rs` instead,
re-export `FileToolbar` through the layout modules, and update its import and
usage in `src/app.rs`. Keep the editor and routes
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

Put this `FileToolbar` in the new `file_toolbar.rs`, importing the prelude and
`crate::draft::Draft`. Replace
`<DataToolbar />` in `NotesApp` with `<FileToolbar />`:

```rust
#[component]
pub(crate) fn FileToolbar() -> ViewResult<impl IntoView> {
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

The selector owns its own full-screen interface. To try it, create
`src/layouts/root/components/file_picker.rs` with the prelude import and this
`pub(crate)` component. Declare and re-export it through the same layout modules
as `FileToolbar`, then import it in `src/app.rs` and insert `<FilePicker />`
after `<FileToolbar />`. The selected path is displayed without replacing the draft:

```rust
#[component]
pub(crate) fn FilePicker() -> impl IntoView {
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

Create `src/layouts/root/components/editor_tools.rs`, importing the prelude
and `crate::draft::Draft`. Declare and re-export `EditorTools` through the layout
modules as above, then import it in `src/app.rs` and insert `<EditorTools />`
after `<FileToolbar />`. Save a note first before trying the saved-file button.
The first button edits the draft signal through a temporary Markdown file;
the second edits the saved file:

```rust
#[component]
pub(crate) fn EditorTools() -> impl IntoView {
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
`file_picker.rs`, `editor_tools.rs`, their module declarations, re-exports,
imports, and shell tags after trying them. The final
application below uses the fixed `notes-data/note.md` path and is the version
to continue with in chapter 11. No additional Cargo dependencies are required.

## Complete files for this chapter

Create or replace only the files listed below; retain all other files from
the previous chapter. Keep the Cargo manifest from [chapter 1](01-getting-started.md).

Delete these superseded files: `src/layouts/root/components/data_toolbar.rs`, `src/services/mod.rs`, `src/services/notes.rs`.

The project now contains:

```text
└── src/
    ├── app.rs
    ├── draft.rs
    ├── layouts/
    │   ├── mod.rs
    │   └── root/
    │       ├── component.rs
    │       ├── components/
    │       │   ├── file_toolbar.rs
    │       │   ├── mod.rs
    │       │   └── note_heading.rs
    │       ├── mod.rs
    │       └── style.rs
    ├── main.rs
    └── pages/
        ├── edit/
        │   ├── mod.rs
        │   └── page.rs
        ├── mod.rs
        ├── not_found/
        │   ├── mod.rs
        │   └── page.rs
        ├── note_details/
        │   ├── mod.rs
        │   └── page.rs
        ├── preview/
        │   ├── mod.rs
        │   └── page.rs
        └── required_file/
            ├── mod.rs
            └── page.rs
```

Run `cargo run` in your application project. Ctrl+Q quits.

### `src/app.rs`

```rust
use leptatui::prelude::*;

use crate::draft::Draft;
use crate::layouts::FileToolbar;
use crate::layouts::RootLayout;
use crate::pages::{NotFound, NoteDetails, NoteEditor, NotePreview, RequiredFile};

#[component]
pub(crate) fn NotesApp() -> impl IntoView {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return KeyControl::Exit;
        }
        KeyControl::Pass
    });
    let title = RwSignal::new(String::from("First note"));
    let body = RwSignal::new(String::from("Learn to build a terminal application."));
    let status = RwSignal::new(String::from("Editing"));

    provide_context(Draft {
        title,
        body,
        status,
    });

    view! {
        <Router initial_path="/edit">
            <RootLayout>
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
            </RootLayout>
        </Router>
    }
}
```

### `src/layouts/mod.rs`

```rust
mod root;

pub(crate) use root::FileToolbar;
pub(crate) use root::RootLayout;
```

### `src/layouts/root/components/file_toolbar.rs`

```rust
use leptatui::prelude::*;

use crate::draft::Draft;

#[component]
pub(crate) fn FileToolbar() -> ViewResult<impl IntoView> {
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

### `src/layouts/root/components/mod.rs`

```rust
mod file_toolbar;
mod note_heading;

pub(crate) use file_toolbar::FileToolbar;
pub(super) use note_heading::NoteHeading;
```

### `src/layouts/root/mod.rs`

```rust
mod component;
mod components;
mod style;

pub(crate) use component::RootLayout;
pub(crate) use components::FileToolbar;
```

### `src/main.rs`

```rust
mod app;
mod draft;
mod layouts;
mod pages;

use leptatui::prelude::*;

use crate::app::NotesApp;

#[tokio::main]
async fn main() -> leptatui::app::Result<()> {
    App::new(view! { <NotesApp /> }).run().await
}
```

[Previous: Async data](09-async-data.md) · [Documentation index](../README.md) · [Next: Documents and media](11-documents-and-media.md)
