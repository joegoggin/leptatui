# 7. Pages and navigation

Continue from [chapter 6: Styling and layout](06-styling-and-layout.md).

You can edit a draft in place. Let's add a Preview page so you can
switch between editing and reading the same text.

The router chooses a view using an in-memory path such as `/edit`.

Move `src/components/note_editor.rs` to `src/pages/edit/page.rs`. The
component stays the same because it already reads the shared draft through
context. Delete the old `src/components/mod.rs` and its now-empty directory.

Create `src/pages/edit/mod.rs` with these contents.

```rust
mod page;

pub(crate) use page::NoteEditor;
```

Create `src/pages/preview/page.rs` with these contents.

```rust
use leptatui::prelude::*;

use crate::draft::Draft;

#[component]
pub(crate) fn NotePreview() -> impl IntoView {
    let Draft { title, body, .. } = expect_context::<Draft>();
    view! {
        <Div>
            <H2>{title}</H2>
            <Paragraph>{body}</Paragraph>
        </Div>
    }
}
```

Create `src/pages/preview/mod.rs` with these contents.

```rust
mod page;

pub(crate) use page::NotePreview;
```

Create `src/pages/not_found/page.rs` with these contents.

```rust
use leptatui::prelude::*;

#[component]
pub(crate) fn NotFound() -> impl IntoView {
    view! { <Text>"Page not found. Choose Edit or Preview."</Text> }
}
```

Create `src/pages/not_found/mod.rs` with these contents.

```rust
mod page;

pub(crate) use page::NotFound;
```

Create `src/pages/mod.rs` with these contents.

```rust
mod edit;
mod not_found;
mod preview;

pub(crate) use edit::NoteEditor;
pub(crate) use not_found::NotFound;
pub(crate) use preview::NotePreview;
```

In `src/main.rs`, remove `mod components;` and add `mod pages;`.

Update `src/layouts/root/component.rs` with these contents.

```rust
use leptatui::prelude::*;

use super::components::NoteHeading;
use super::style::use_root_layout_styles;

#[component]
pub(crate) fn RootLayout(children: Children) -> impl IntoView {
    use_root_layout_styles();

    view! {
        <Div class="notes-shell">
            <NoteHeading text="My notes" />
            {children}
        </Div>
    }
}
```

In `src/layouts/root/style.rs`, add this rule inside `stylesheet!` alongside the heading rule.

```rust
A:active { fg: Color::LightGreen; }
```

In `src/app.rs`, replace `use crate::components::NoteEditor;` with this import.

```rust
use crate::pages::{NotFound, NoteEditor, NotePreview};
```

Replace the final `view!` in `NotesApp` with the router.

```rust
view! {
    <Router initial_path="/edit">
        <RootLayout>
            <Div class="notes-shell__actions">
                <A href="/edit">"Edit"</A>
                <A href="/preview">"Preview"</A>
            </Div>
            <Routes fallback=NotFound>
                <Route path="/edit" view=NoteEditor />
                <Route path="/preview" view=NotePreview />
            </Routes>
        </RootLayout>
    </Router>
}
```

`A` is an internal route anchor. `Link` opens a URL or local file with the
system handler. The shell outside `Routes` stays mounted, so its context-owned
draft survives navigation. Each route points to the component re-exported by
its page module.

Run `cargo run` and edit the body. Tab to Preview and press Enter. It should show the latest draft; returning to
Edit should preserve the text. An active route anchor matches `A:active`.
Use `exact=true` when a root anchor should match only its exact destination.

## Programmatic navigation and history

Inside a component below `Router`, call `use_navigate()` to get a navigation
function. For example, a button can call
`navigate("/preview", NavigateOptions::default())` and return `()` to keep
the app running. `use_location()` gives reactive pathname and search state. `use_history()` exposes back and forward operations; history is local
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

## Complete files for this chapter

Create or replace only the files listed below; retain all other files from
the previous chapter. Keep the Cargo manifest from [chapter 1](01-getting-started.md).

Delete these superseded files: `src/components/mod.rs`, `src/components/note_editor.rs`.

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
        └── preview/
            ├── mod.rs
            └── page.rs
```

Run `cargo run` in your application project. Ctrl+Q quits.

### `src/app.rs`

```rust
use leptatui::prelude::*;

use crate::draft::Draft;
use crate::layouts::RootLayout;
use crate::pages::{NotFound, NoteEditor, NotePreview};

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
                </Div>
                <Routes fallback=NotFound>
                    <Route path="/edit" view=NoteEditor />
                    <Route path="/preview" view=NotePreview />
                </Routes>
            </RootLayout>
        </Router>
    }
}
```

### `src/layouts/root/component.rs`

```rust
use leptatui::prelude::*;

use super::components::NoteHeading;
use super::style::use_root_layout_styles;

#[component]
pub(crate) fn RootLayout(children: Children) -> impl IntoView {
    use_root_layout_styles();

    view! {
        <Div class="notes-shell">
            <NoteHeading text="My notes" />
            {children}
        </Div>
    }
}
```

### `src/layouts/root/style.rs`

```rust
use leptatui::prelude::*;

pub(super) fn use_root_layout_styles() {
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
    };
}
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

### `src/pages/edit/mod.rs`

```rust
mod page;

pub(crate) use page::NoteEditor;
```

### `src/pages/edit/page.rs`

```rust
use leptatui::prelude::*;

use crate::draft::Draft;

#[component]
pub(crate) fn NoteEditor() -> impl IntoView {
    let Draft {
        title,
        body,
        status,
    } = expect_context::<Draft>();

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
```

### `src/pages/mod.rs`

```rust
mod edit;
mod not_found;
mod preview;

pub(crate) use edit::NoteEditor;
pub(crate) use not_found::NotFound;
pub(crate) use preview::NotePreview;
```

### `src/pages/not_found/mod.rs`

```rust
mod page;

pub(crate) use page::NotFound;
```

### `src/pages/not_found/page.rs`

```rust
use leptatui::prelude::*;

#[component]
pub(crate) fn NotFound() -> impl IntoView {
    view! { <Text>"Page not found. Choose Edit or Preview."</Text> }
}
```

### `src/pages/preview/mod.rs`

```rust
mod page;

pub(crate) use page::NotePreview;
```

### `src/pages/preview/page.rs`

```rust
use leptatui::prelude::*;

use crate::draft::Draft;

#[component]
pub(crate) fn NotePreview() -> impl IntoView {
    let Draft { title, body, .. } = expect_context::<Draft>();
    view! {
        <Div>
            <H2>{title}</H2>
            <Paragraph>{body}</Paragraph>
        </Div>
    }
}
```

[Previous: Styling and layout](06-styling-and-layout.md) · [Documentation index](../README.md) · [Next: Handling failures](08-handling-failures.md)
