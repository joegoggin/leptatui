# 5. Reusable components

Continue from [chapter 4: Editing notes](04-editing-notes.md).

The editor works inside one function. Let's give the form and shell
separate components that can share the same draft.

## Pass inputs with props

A component's function parameters become props: values supplied by its parent.
Declare data props with their value type, such as `String`. `#[component]`
automatically wraps them in `Prop<T>` inside the component body. Render the
handle directly in `view!`, or read its current value with `.get()` or `.with()`;
`view!` automatically tracks those reads. Callers
can write `<NoteHeading text={title.get()} />` without a tracking closure.
Component setup runs once, so prop changes preserve local signals and state.

The root layout will own the heading and use `Children` to place the editor
inside a border. First define the signal handles the editor will share.

Create `src/draft.rs` with these contents.

```rust
use leptatui::prelude::*;

#[derive(Clone, Copy)]
pub(crate) struct Draft {
    pub(crate) title: RwSignal<String>,
    pub(crate) body: RwSignal<String>,
    pub(crate) status: RwSignal<String>,
}
```

Create `src/layouts/root/components/note_heading.rs` with these contents.

```rust
use leptatui::prelude::*;

#[component]
pub(in crate::layouts::root) fn NoteHeading(#[prop(into)] text: String) -> impl IntoView {
    view! { <H1>{text}</H1> }
}
```

Create `src/layouts/root/component.rs` with these contents.

```rust
use leptatui::prelude::*;

use super::components::NoteHeading;

#[component]
pub(crate) fn RootLayout(children: Children) -> impl IntoView {
    view! {
        <Block>
            <Div>
                <NoteHeading text="My notes" />
                {children}
            </Div>
        </Block>
    }
}
```

Create `src/layouts/root/components/mod.rs` with these contents.

```rust
mod note_heading;

pub(super) use note_heading::NoteHeading;
```

Create `src/layouts/root/mod.rs` with these contents.

```rust
mod component;
mod components;

pub(crate) use component::RootLayout;
```

Create `src/layouts/mod.rs` with these contents.

```rust
mod root;

pub(crate) use root::RootLayout;
```

`H1` is the highest-level semantic heading; chapter 11 introduces the full
document component set. `#[prop(into)]` permits inputs convertible into the
requested value type, so a caller
can write `<NoteHeading text="My notes" />`. `#[prop(optional)]` allows a prop
to use its type's default; `#[prop(default = expression)]` supplies a chosen
default. `Children` is a callable holding nested content.

`#[component]` retains `Children` lazily and exposes a repeatable handle inside
the component. Render it directly with `{children}`; the callable form
`{children()}` remains supported. Both forms preserve the original child nodes
and their local state.
Use `ChildrenFn` or `ChildrenMut` for explicitly repeatable or mutable factories.
Mark event props with `#[prop(callback)]`; mark intentional setup-only inputs
with `#[prop(static)]`. These inputs keep their declared types and are retained
rather than automatically wrapped. Explicit `Prop<T>` data parameters remain
supported without an additional wrapper.

Inside the body, a data prop is a handle: use `text.get().to_uppercase()` for
string operations. Values computed during setup remain snapshots; put those
reads inside `view!` to make the rendered result reactive.

## Share the draft through context

Move the chapter 4 form, status, and keyboard instructions into a new
`NoteEditor` component in `src/components/note_editor.rs`. Import
`crate::draft::Draft` and the Leptatui prelude in that file. Read the shared draft
at the start of the component, then use its signals in the form.

```rust
use leptatui::prelude::*;

use crate::draft::Draft;

#[component]
pub(crate) fn NoteEditor() -> impl IntoView {
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
```

In `src/app.rs`, import `Draft`, `RootLayout`, and `NoteEditor`, then replace
the existing `NotesApp` with the following definition. It keeps the
quit handler and creates the draft signals, then provides them before returning
the view. The heading stays in `RootLayout`, above the editor. Add these imports below the prelude import, then replace
the component.

```rust
use crate::components::NoteEditor;
use crate::draft::Draft;
use crate::layouts::RootLayout;
```

```rust
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

    provide_context(Draft { title, body, status });

    view! {
        <RootLayout>
            <NoteEditor />
        </RootLayout>
    }
}
```

`NotesApp` owns the signals. `expect_context::<Draft>()` retrieves handles to
those same signals in `NoteEditor`, so editing the form updates the shared
draft rather than creating a separate copy of its contents.

Context stores a value by Rust type for descendants to retrieve. Use
`expect_context` when it must exist, or `use_context::<Draft>()` when absence
is valid. Keep shared signals owned by `NotesApp` so replacing editor pages
will not destroy the draft. Props are useful for explicit parent-to-child
configuration; context is useful across many descendants.

Create `src/components/mod.rs` with these contents.

```rust
mod note_editor;

pub(crate) use note_editor::NoteEditor;
```

In `src/main.rs`, add these lines alongside `mod app;`.

```rust
mod components;
mod draft;
mod layouts;
```

Run `cargo run`. Tab to the title, press `i`, and type a few characters.
You can still edit the draft, now inside the bordered layout.

## Collections and replacement

The following optional experiment is separate from the notes app. Inside a
component, a container can accept a vector of views; return `list` to display it:

```rust
let titles = ["First note", "Second note"];
let rows = titles.into_iter()
    .map(|title| view! { <Text>{title}</Text> })
    .collect::<Vec<_>>();
let list = view! { <Div>{rows}</Div> };
```

`IntoViews` accepts homogeneous collections and mixed tuples. Children are
stored as `AnyView` when entering the tree. A tracked child closure inside
`view!` can return different views as its inputs change.

Use `keyed(move || selected_id.get(), move || view! { <NoteEditor /> })` when
changing a selection should mount a fresh editor and reset its local view
state. The key closure tracks dependencies; the child factory is untracked.
This is subtree replacement, not automatic keyed-list diffing. Ordinary controlled inputs retain their editing state automatically.

## Complete files for this chapter

Create or replace only the files listed below; retain all other files from
the previous chapter. Keep the Cargo manifest from [chapter 1](01-getting-started.md).

The project now contains:

```text
└── src/
    ├── app.rs
    ├── components/
    │   ├── mod.rs
    │   └── note_editor.rs
    ├── draft.rs
    ├── layouts/
    │   ├── mod.rs
    │   └── root/
    │       ├── component.rs
    │       ├── components/
    │       │   ├── mod.rs
    │       │   └── note_heading.rs
    │       └── mod.rs
    └── main.rs
```

Run `cargo run` in your application project. Ctrl+Q quits.

### `src/app.rs`

```rust
use leptatui::prelude::*;

use crate::components::NoteEditor;
use crate::draft::Draft;
use crate::layouts::RootLayout;

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
        <RootLayout>
            <NoteEditor />
        </RootLayout>
    }
}
```

### `src/components/mod.rs`

```rust
mod note_editor;

pub(crate) use note_editor::NoteEditor;
```

### `src/components/note_editor.rs`

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

### `src/draft.rs`

```rust
use leptatui::prelude::*;

#[derive(Clone, Copy)]
pub(crate) struct Draft {
    pub(crate) title: RwSignal<String>,
    pub(crate) body: RwSignal<String>,
    pub(crate) status: RwSignal<String>,
}
```

### `src/layouts/mod.rs`

```rust
mod root;

pub(crate) use root::RootLayout;
```

### `src/layouts/root/component.rs`

```rust
use leptatui::prelude::*;

use super::components::NoteHeading;

#[component]
pub(crate) fn RootLayout(children: Children) -> impl IntoView {
    view! {
        <Block>
            <Div>
                <NoteHeading text="My notes" />
                {children}
            </Div>
        </Block>
    }
}
```

### `src/layouts/root/components/mod.rs`

```rust
mod note_heading;

pub(super) use note_heading::NoteHeading;
```

### `src/layouts/root/components/note_heading.rs`

```rust
use leptatui::prelude::*;

#[component]
pub(in crate::layouts::root) fn NoteHeading(#[prop(into)] text: String) -> impl IntoView {
    view! { <H1>{text}</H1> }
}
```

### `src/layouts/root/mod.rs`

```rust
mod component;
mod components;

pub(crate) use component::RootLayout;
```

### `src/main.rs`

```rust
mod app;
mod components;
mod draft;
mod layouts;

use leptatui::prelude::*;

use crate::app::NotesApp;

#[tokio::main]
async fn main() -> leptatui::app::Result<()> {
    App::new(view! { <NotesApp /> }).run().await
}
```

[Previous: Editing notes](04-editing-notes.md) · [Documentation index](../README.md) · [Next: Styling and layout](06-styling-and-layout.md)
