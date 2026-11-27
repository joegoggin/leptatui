# 8. Handling failures

Continue from [chapter 7: Pages and navigation](07-pages-and-navigation.md).

Your router can show Edit and Preview. Let's add two pages that let
you see what happens when a page can't open.

A component returning
`ViewResult<impl IntoView>` can use `?` during setup. The component macro wraps
its final bare view expression in `Ok` for you. The parameter and query models
derive `Clone` because the tracked view captures their values.

Create these empty files, including any directories they need.

- `src/pages/note_details/mod.rs`
- `src/pages/note_details/page.rs`
- `src/pages/required_file/mod.rs`
- `src/pages/required_file/page.rs`

Add the following to `src/pages/mod.rs`.

```rust
mod note_details;
mod required_file;
```

Add the following to `src/pages/note_details/mod.rs`.

```rust
mod page;
```

Add the following to `src/pages/required_file/mod.rs`.

```rust
mod page;
```

Fill in `src/pages/note_details/page.rs` for the following page and its private
parameter and query models. You’ll connect it to `/notes/:id` below.

```rust
use leptatui::prelude::*;

#[derive(Clone, RouteParams)]
struct NoteParams {
    id: usize,
}

#[derive(Clone, QueryParams)]
struct NoteQuery {
    line: Option<usize>,
}

#[component]
pub(crate) fn NoteDetails() -> ViewResult<impl IntoView> {
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
message needs application context. Fill in
`src/pages/required_file/page.rs` with the following code.

```rust
use leptatui::prelude::*;

#[component]
pub(crate) fn RequiredFile() -> ViewResult<impl IntoView> {
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

Import both pages in `src/app.rs`. Keep the parameter/query models private
to the details page. In the application navigation `Div`, after Preview, add:

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

Expose `NoteDetails` from its page module in `src/pages/note_details/mod.rs`.
The full file should now look like this.

```rust
mod page;

pub(crate) use page::NoteDetails;
```

Expose `RequiredFile` from its page module in `src/pages/required_file/mod.rs`.
The full file should now look like this.

```rust
mod page;

pub(crate) use page::RequiredFile;
```

Make both new pages available alongside the existing ones in `src/pages/mod.rs`.
The full file should now look like this.

```rust
mod edit;
mod not_found;
mod note_details;
mod preview;
mod required_file;

pub(crate) use edit::NoteEditor;
pub(crate) use not_found::NotFound;
pub(crate) use note_details::NoteDetails;
pub(crate) use preview::NotePreview;
pub(crate) use required_file::RequiredFile;
```

In `src/app.rs`, replace the page import.

```rust
use crate::pages::{NotFound, NoteDetails, NoteEditor, NotePreview, RequiredFile};
```

Run `cargo run` and open **Note details**, then **Invalid note**. The latter
should display the error screen; press `b` to return to the previous page.
**Read note.md** reads a separate `note.md` in your working directory. If it
does not exist, this intentionally demonstrates an error with context. This
file is separate from the `notes-data/note.md` introduced in chapter 10.
Keep these routes in subsequent chapters.

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

### `src/pages/mod.rs`

```rust
mod edit;
mod not_found;
mod note_details;
mod preview;
mod required_file;

pub(crate) use edit::NoteEditor;
pub(crate) use not_found::NotFound;
pub(crate) use note_details::NoteDetails;
pub(crate) use preview::NotePreview;
pub(crate) use required_file::RequiredFile;
```

### `src/pages/note_details/mod.rs`

```rust
mod page;

pub(crate) use page::NoteDetails;
```

### `src/pages/note_details/page.rs`

```rust
use leptatui::prelude::*;

#[derive(Clone, RouteParams)]
struct NoteParams {
    id: usize,
}

#[derive(Clone, QueryParams)]
struct NoteQuery {
    line: Option<usize>,
}

#[component]
pub(crate) fn NoteDetails() -> ViewResult<impl IntoView> {
    let params = use_params::<NoteParams>()?;
    let query = use_query::<NoteQuery>()?;
    view! { <Text>{format!("Note {}, line {:?}", params.id, query.line)}</Text> }
}
```

### `src/pages/required_file/mod.rs`

```rust
mod page;

pub(crate) use page::RequiredFile;
```

### `src/pages/required_file/page.rs`

```rust
use leptatui::prelude::*;

#[component]
pub(crate) fn RequiredFile() -> ViewResult<impl IntoView> {
    let source = match std::fs::read_to_string("note.md") {
        Ok(source) => source,
        Err(error) => view_error!(error => "Could not load note.md"),
    };
    view! { <Paragraph>{source}</Paragraph> }
}
```

[Previous: Pages and navigation](07-pages-and-navigation.md) · [Documentation index](../README.md) · [Next: Async data](09-async-data.md)
