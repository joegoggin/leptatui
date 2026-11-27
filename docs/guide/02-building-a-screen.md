# 2. Building a screen

Prerequisites: [Chapter 1: Getting started](01-getting-started.md).

Build a static note and learn how the view tree describes its structure.
Keep `src/main.rs` from chapter 1. In `src/app.rs`, keep the imports and
quit handler, and replace only the `view!` expression in `NotesApp` with:

```rust
view! {
    <Block>
        <Div>
            <Text>"My notes"</Text>
            <Text>"First note"</Text>
            <Paragraph>"Learn to build a terminal application."</Paragraph>
            <Text>"Ctrl+Q quits"</Text>
        </Div>
    </Block>
}
```

A view is a piece of the interface. `Div` groups multiple children and stacks
them vertically by default. `Block` adds a border around one child; that child
can be a `Div` containing the rest of the screen. `Text` displays text, while
`Paragraph` provides semantic body text with wrapping.

Tags begin with capitals. Literal content is quoted, and braces insert a Rust
expression. For example, `<Text>{String::from("First note")}</Text>` inserts a
computed string. These tags produce Leptatui terminal views, not HTML.

## Use macros for application UI

Build interfaces with `view!` and declare their styles with `stylesheet!`.
These macros are the intended application authoring interface: they abstract
view construction and stylesheet registration and keep structure and styling
consistent across components. Direct view constructors and fluent style
builders are low-level escape hatches; using them for ordinary application UI
is an anti-pattern. The tutorials use macros throughout.

Run the app. You should see the note inside a border, with each text child
below the previous one. Resize the terminal to see paragraph wrapping.

The [component reference](../reference/components.md) lists the available
views. We will introduce them as the application needs them.

## Complete files for this chapter

Create or replace only the files listed below; retain all other files from
the previous chapter. Keep the Cargo manifest from [chapter 1](01-getting-started.md).

Run `cargo run` in your application project. Ctrl+Q quits.

### `src/app.rs`

```rust
use leptatui::prelude::*;

#[component]
pub(crate) fn NotesApp() -> impl IntoView {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return KeyControl::Exit;
        }
        KeyControl::Pass
    });

    view! {
        <Block>
            <Div>
                <Text>"My notes"</Text>
                <Text>"First note"</Text>
                <Paragraph>"Learn to build a terminal application."</Paragraph>
                <Text>"Ctrl+Q quits"</Text>
            </Div>
        </Block>
    }
}
```

[Previous: Getting started](01-getting-started.md) · [Documentation index](../README.md) · [Next: State and interaction](03-state-and-interaction.md)
