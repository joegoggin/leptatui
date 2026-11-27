# 9. Async data

Prerequisites: [Chapter 8: Handling failures](08-handling-failures.md).

Add a mock service without requiring a network, account, or backend. Continue
from chapter 8 and add a data toolbar to the routed editor shell.

## Read with a resource

A resource runs an async fetcher when its reactive source changes or when you
call `refetch`. Put the mock fetch in `src/services/notes.rs` as
`pub(crate) async fn load_note() -> Result<String, String>`. Declare the module
through `src/services/mod.rs` and `mod services;` in `main.rs`. The toolbar
imports `load_note` and creates a resource during component setup:

```rust
let loaded = Resource::new(|| (), |_| load_note());
```

`get()` returns `None` before completion, then `Some(output)`. The output can
itself be `Ok(value)` or `Err(error)`. `is_loading()` is separate from that
value: reloading retains the previous result. Older requests cannot overwrite
the result of a newer request.

The toolbar deliberately requires **Use loaded note** before replacing the
draft. A background read should not unexpectedly erase text the user is typing.
**Reload** fetches again; it does not apply the result automatically.

## Write with an action

An action starts only when you dispatch an input. Put the mock validation and
save delay in `pub(crate) async fn save_note(body: String) -> Result<String, String>`
in the same service module. Clone borrowed input before passing ownership to
its future:

```rust
let save = Action::new(|body: &String| save_note(body.clone()));
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

Create `src/layouts/root/components/data_toolbar.rs` and import `Draft` plus
`load_note` and `save_note`. The toolbar owns resource/action handles, callbacks,
and pending/result presentation; the service owns the mock data operations.

Re-export `DataToolbar` through the component and layout modules, then import
`crate::layouts::DataToolbar` in `src/app.rs`. In `NotesApp`, insert
`<DataToolbar />` immediately before `<Routes fallback=NotFound>`. Keep the
routes and shared draft from chapter 8.

## Complete files for this chapter

Create or replace only the files listed below; retain all other files from
the previous chapter. Keep the Cargo manifest from [chapter 1](01-getting-started.md).

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
    │       │   ├── data_toolbar.rs
    │       │   ├── mod.rs
    │       │   └── note_heading.rs
    │       ├── mod.rs
    │       └── style.rs
    ├── main.rs
    ├── pages/
    │   ├── edit/
    │   │   ├── mod.rs
    │   │   └── page.rs
    │   ├── mod.rs
    │   ├── not_found/
    │   │   ├── mod.rs
    │   │   └── page.rs
    │   ├── note_details/
    │   │   ├── mod.rs
    │   │   └── page.rs
    │   ├── preview/
    │   │   ├── mod.rs
    │   │   └── page.rs
    │   └── required_file/
    │       ├── mod.rs
    │       └── page.rs
    └── services/
        ├── mod.rs
        └── notes.rs
```

Run `cargo run` in your application project. Ctrl+Q quits.

### `src/app.rs`

```rust
use leptatui::prelude::*;

use crate::draft::Draft;
use crate::layouts::DataToolbar;
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
                <DataToolbar />
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

pub(crate) use root::DataToolbar;
pub(crate) use root::RootLayout;
```

### `src/layouts/root/components/data_toolbar.rs`

```rust
use leptatui::prelude::*;

use crate::draft::Draft;
use crate::services::notes::{load_note, save_note};

#[component]
pub(crate) fn DataToolbar() -> impl IntoView {
    let Draft { body, status, .. } = expect_context::<Draft>();
    let loaded = Resource::new(|| (), |_| load_note());
    let save = Action::new(|body: &String| save_note(body.clone()));
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

### `src/layouts/root/components/mod.rs`

```rust
mod data_toolbar;
mod note_heading;

pub(crate) use data_toolbar::DataToolbar;
pub(super) use note_heading::NoteHeading;
```

### `src/layouts/root/mod.rs`

```rust
mod component;
mod components;
mod style;

pub(crate) use component::RootLayout;
pub(crate) use components::DataToolbar;
```

### `src/main.rs`

```rust
mod app;
mod draft;
mod layouts;
mod pages;
mod services;

use leptatui::prelude::*;

use crate::app::NotesApp;

#[tokio::main]
async fn main() -> leptatui::app::Result<()> {
    App::new(view! { <NotesApp /> }).run().await
}
```

### `src/services/mod.rs`

```rust
pub(crate) mod notes;
```

### `src/services/notes.rs`

```rust
pub(crate) async fn load_note() -> Result<String, String> {
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    Ok(String::from("A note from the mock service"))
}

pub(crate) async fn save_note(body: String) -> Result<String, String> {
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    if body.trim().is_empty() {
        Err(String::from("Write something before saving"))
    } else {
        Ok(format!("Accepted {} bytes", body.len()))
    }
}
```

[Previous: Handling failures](08-handling-failures.md) · [Documentation index](../README.md) · [Next: Files and editors](10-files-and-editors.md)
