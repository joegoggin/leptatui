# 5. Reusable components

Prerequisites: [Chapter 4: Editing notes](04-editing-notes.md).

Split the editor from the application shell without losing the draft.
The final listing below combines the components and application shell.

## Pass inputs with props

A component's function parameters become props: values supplied by its parent.
Declare data props with their value type, such as `String`. `#[component]`
automatically wraps them in `Prop<T>` inside the component body. Render the
handle directly in `view!`, or read its current value with `.get()` or `.with()`;
`view!` automatically tracks those reads. Callers
can write `<NoteHeading text={title.get()} />` without a tracking closure.
Component setup runs once, so prop changes preserve local signals and state.

Add these definitions above `NotesApp`:

```rust
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
fn Panel(children: Children) -> impl IntoView {
    view! { <Block><Div>{children}</Div></Block> }
}
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
`NoteEditor` component above `NotesApp`. Read the shared draft at the start of
the component, then use its signals in the form:

```rust
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
```

Replace the existing `NotesApp` with the following definition. It keeps the
quit handler and creates the draft signals, then provides them before returning
the view. The heading stays in the shell, above the editor:

```rust
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

    view! {
        <Panel>
            <NoteHeading text="My notes" />
            <NoteEditor />
        </Panel>
    }
}
```

`NotesApp` owns the signals. `expect_context::<Draft>()` retrieves handles to
those same signals in `NoteEditor`, so editing the form updates the shared
draft rather than creating a separate copy of its contents.

Context stores a value by Rust type for descendants to retrieve. Use
`expect_context` when it must exist, or `use_context::<Draft>()` when absence
is valid. Keep shared signals owned by the shell so replacing editor pages
will not destroy the draft. Props are useful for explicit parent-to-child
configuration; context is useful across many descendants.

## Collections and replacement

The following optional experiment is separate from the notes app. Inside a
component, a container can accept a vector of views; return `list` to display it:

```rust
let titles = ["First note", "Second note"];
let rows = titles.into_iter().map(text).collect::<Vec<_>>();
let list = view! { <Div>{rows}</Div> };
```

`IntoViews` accepts homogeneous collections and mixed tuples. Children are
stored as `AnyView` when entering the tree. `dynamic(move || ...)` is the
builder equivalent of a tracked child closure and can return different view
types as its inputs change.

Use `keyed(move || selected_id.get(), move || view! { <NoteEditor /> })` when
changing a selection should mount a fresh editor and reset its local view
state. The key closure tracks dependencies; the child factory is untracked.
This is subtree replacement, not automatic keyed-list diffing. Ordinary controlled inputs retain their editing state automatically.

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
fn Panel(children: Children) -> impl IntoView {
    view! {
        <Block>
            <Div>{children}</Div>
        </Block>
    }
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

    view! {
        <Panel>
            <NoteHeading text="My notes" />
            <NoteEditor />
        </Panel>
    }
}

#[tokio::main]
async fn main() -> leptatui::app::Result<()> {
    App::new(view! { <NotesApp /> }).run().await
}
```

[Previous: Editing notes](04-editing-notes.md) · [Documentation index](../README.md) · [Next: Styling and layout](06-styling-and-layout.md)
