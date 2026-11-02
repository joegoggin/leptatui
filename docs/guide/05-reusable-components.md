# 5. Reusable components

Prerequisites: [Chapter 4: Editing notes](04-editing-notes.md).

Split the editor from the application shell without losing the draft.
The [routed checkpoint](../checkpoints/07-routed-notes.md) will bring these
pieces together as a complete file.

## Pass inputs with props

A component's function parameters become props: values supplied by its parent.
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
    view! { <Block><Div>{children()}</Div></Block> }
}
```

`H1` is the highest-level semantic heading; chapter 11 introduces the full
document component set. `#[prop(into)]` permits inputs convertible into the requested type, so a caller
can write `<NoteHeading text="My notes" />`. `#[prop(optional)]` allows a prop
to use its type's default; `#[prop(default = expression)]` supplies a chosen
default. `Children` is a callable holding nested content. Use `ChildrenFn` or
`ChildrenMut` when you need repeatable `Fn` or mutable `FnMut` children.

## Share the draft through context

Move the chapter 4 form into a new `NoteEditor` component. At the start of that
component, read:

```rust
let Draft { title, body, status } = expect_context::<Draft>();
```

In `NotesApp`, create the signals as before, then provide the draft before
returning the view:

```rust
provide_context(Draft { title, body, status });
view! { <Panel><NoteHeading text="My notes" /><NoteEditor /></Panel> }
```

Context stores a value by Rust type for descendants to retrieve. Use
`expect_context` when it must exist, or `use_context::<Draft>()` when absence
is valid. Keep shared signals owned by the shell so replacing editor pages
will not destroy the draft. Props are useful for explicit parent-to-child
configuration; context is useful across many descendants.

## Collections and replacement

For a list of titles, a container can accept a vector of views:

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
This is subtree replacement, not automatic keyed-list diffing. Keep ordinary
controlled editing inside a dynamic boundary when cursor retention matters.

[Previous: Editing notes](04-editing-notes.md) · [Documentation index](../README.md) · [Next: Styling and layout](06-styling-and-layout.md)
