# 5. Reusable components

Prerequisites: [Chapter 4: Editing notes](04-editing-notes.md).

Split the editor from the application shell without losing the draft.
The [routed checkpoint](../checkpoints/07-routed-notes.md) will bring these
pieces together as a complete file.

## Pass inputs with props

A component's function parameters become props: values supplied by its parent.
Use `Prop<T>` for changing data. The component reads its current value with
`.get()` or `.with()`, and `view!` automatically tracks those reads. Callers
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
fn NoteHeading(#[prop(into)] text: Prop<String>) -> impl IntoView {
    view! { <H1>{text.get()}</H1> }
}

#[component]
fn Panel(children: Children) -> impl IntoView {
    view! { <Block><Div>{children()}</Div></Block> }
}
```

`H1` is the highest-level semantic heading; chapter 11 introduces the full
document component set. `#[prop(into)]` permits inputs convertible into the
requested value type, so a caller
can write `<NoteHeading text="My notes" />`. `#[prop(optional)]` allows a prop
to use its type's default; `#[prop(default = expression)]` supplies a chosen
default. `Children` is a callable holding nested content.

`#[component]` retains `Children` lazily and exposes a repeatable handle inside
the component, so `{children()}` works without rebuilding the child nodes.
Use `ChildrenFn` or `ChildrenMut` for explicitly repeatable or mutable factories.
Mark event props with `#[prop(callback)]`; mark intentional setup-only inputs
with `#[prop(static)]`. These inputs are retained rather than tracked.

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
This is subtree replacement, not automatic keyed-list diffing. Ordinary controlled inputs retain their editing state automatically.

[Previous: Editing notes](04-editing-notes.md) · [Documentation index](../README.md) · [Next: Styling and layout](06-styling-and-layout.md)
