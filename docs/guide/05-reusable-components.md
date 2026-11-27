# 5. Reusable components

Continue from [chapter 4: Editing notes](04-editing-notes.md).

The editor works inside one function. Let's give the form and shell
separate components that can share the same draft.

Create these empty files, including any directories they need.

- `src/components/mod.rs`
- `src/components/note_editor.rs`
- `src/draft.rs`
- `src/layouts/mod.rs`
- `src/layouts/root/mod.rs`
- `src/layouts/root/component.rs`
- `src/layouts/root/components/mod.rs`
- `src/layouts/root/components/note_heading.rs`

Add the following to `src/main.rs`.

```rust
mod components;
mod draft;
mod layouts;
```

Add the following to `src/components/mod.rs`.

```rust
mod note_editor;
```

Add the following to `src/layouts/mod.rs`.

```rust
mod root;
```

Add the following to `src/layouts/root/mod.rs`.

```rust
mod component;
mod components;
```

Add the following to `src/layouts/root/components/mod.rs`.

```rust
mod note_heading;
```

## Pass inputs with props

The editor currently displays “My notes” with a `Text` view. We'll move
that heading into its own component, `NoteHeading`, so the layout can
choose what it says.

Fill in `src/layouts/root/components/note_heading.rs` with these contents.

```rust
use leptatui::prelude::*;

#[component]
pub(in crate::layouts::root) fn NoteHeading(#[prop(into)] text: String) -> impl IntoView {
    view! { <H1>{text}</H1> }
}
```

The `text` parameter is a prop, an input the parent gives to a component.
In `NoteHeading`, `{text}` displays that input inside `H1`, the
highest-level heading. The `#[prop(into)]` attribute lets the parent pass
a string literal such as `"My notes"` for the `String` parameter.

Inside the component, `#[component]` turns the parameter into a
`Prop<String>` that follows changes to the input. You can display it
directly with `{text}` or read its current value with `.get()` or `.with()`
inside `view!`.

Make `NoteHeading` available to the layout from `src/layouts/root/components/mod.rs`.
The full file should now look like this.

```rust
mod note_heading;

pub(super) use note_heading::NoteHeading;
```

Now fill in the layout that uses the heading. `Children` is the content
you'll put between the opening and closing `RootLayout` tags. Rendering
`{children}` below the heading puts the editor inside the same border.

Fill in `src/layouts/root/component.rs` with these contents.

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

Expose `RootLayout` from the root module in `src/layouts/root/mod.rs`.
The full file should now look like this.

```rust
mod component;
mod components;

pub(crate) use component::RootLayout;
```

Make `RootLayout` available as `crate::layouts::RootLayout` from `src/layouts/mod.rs`.
The full file should now look like this.

```rust
mod root;

pub(crate) use root::RootLayout;
```

`RootLayout` gives `NoteHeading` the same title every time, `"My notes"`.
If you wanted the heading to show the draft title, you could pass
`<NoteHeading text={title.get()} />`. Leptatui updates the heading when the
signal changes and keeps the component's existing state. To display the title
in uppercase, write `{text.get().to_uppercase()}` inside `view!`. That puts
the read where Leptatui can track it. Doing the same calculation during
component setup would capture the initial title.

`{children}` marks the spot where the editor will appear. Leptatui holds the
nested content until the layout renders it. You can also write `{children()}`.
Both forms reuse the same child views and preserve their local state.

When you write your own components, you may need one of these options.

- Use `#[prop(optional)]` to let callers omit a prop and use its type's
  default. Use `#[prop(default = expression)]` to choose the default yourself.
- Use `#[prop(callback)]` for an event handler passed by the parent and
  `#[prop(static)]` for an input used only during setup. Both keep the types
  you declare. You can also declare a data prop as `Prop<T>` directly.
- Use `ChildrenFn` for a child factory you want to call repeatedly or
  `ChildrenMut` for a factory that needs mutable access to what it captures.

## Share the draft through context

Both the app and the editor need access to the same title, body, and status.
Group those three signal handles in a `Draft` so the app can share them
together.

Fill in `src/draft.rs` with these contents.

```rust
use leptatui::prelude::*;

#[derive(Clone, Copy)]
pub(crate) struct Draft {
    pub(crate) title: RwSignal<String>,
    pub(crate) body: RwSignal<String>,
    pub(crate) status: RwSignal<String>,
}
```

Move the chapter 4 form, status, and keyboard instructions into a new
`NoteEditor` component in `src/components/note_editor.rs`. Import
`crate::draft::Draft` and the Leptatui prelude in that file. Read the shared draft
at the start of the component, then use its signals in the form.

```rust
use leptatui::prelude::*;

use crate::draft::Draft;

#[component]
pub(crate) fn NoteEditor() -> impl IntoView {
    let Draft {
        title,
        body,
        status
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
                <Input
                    value=title
                    placeholder="Title"
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
            <Text>
                "Tab focuses. i edits. Esc returns to normal mode. Enter submits in normal mode. Ctrl+Q quits."
            </Text>
        </Div>
    }
}
```

Make `NoteEditor` available as `crate::components::NoteEditor` from `src/components/mod.rs`.
The full file should now look like this.

```rust
mod note_editor;

pub(crate) use note_editor::NoteEditor;
```

Replace `src/app.rs` with the file below. It imports `Draft`, `RootLayout`,
and `NoteEditor`. Inside `NotesApp`, keep the Ctrl+Q handler and create the
`title`, `body`, and `status` signals. Then call `provide_context` with the
`Draft` so `NoteEditor` can read those same signals. The `view!` block places
`NoteEditor` inside `RootLayout`, which renders the heading above the editor.

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

`NotesApp` owns the signals. `expect_context::<Draft>()` retrieves handles to
those same signals in `NoteEditor`, so editing the form updates the shared
draft rather than creating a separate copy of its contents.

Context stores a value by Rust type for descendants to retrieve. Use
`expect_context` when it must exist, or `use_context::<Draft>()` when absence
is valid. Keep shared signals owned by `NotesApp` so replacing editor pages
will not destroy the draft. Props are useful for explicit parent-to-child
configuration; context is useful across many descendants.

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
                <Input
                    value=title
                    placeholder="Title"
                    on_input=move || {
                        status.set(String::from("Editing"));
                    }
                />
                <Text>"Body"</Text>
                <TextArea
                    value=body
                    placeholder="Write a note"
                    on_input=move || {
                        status.set(String::from("Editing"));
                    }
                />
            </Form>
            <Text>{status}</Text>
            <Text>
                "Tab focuses. i edits. Esc returns to normal mode. Enter submits in normal mode. Ctrl+Q quits."
            </Text>
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
