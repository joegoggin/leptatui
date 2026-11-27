# 2. Building a screen

Continue from [chapter 1: Getting started](01-getting-started.md).

Your app can display a line of text. Let’s put a title and body inside
a border by nesting views inside one another.

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

Tag names start with a capital letter. To display text as written, put it in
double quotes, as in `<Text>"First note"</Text>`. To display the result of a
Rust expression, put the expression in braces, as in
`<Text>{String::from("First note")}</Text>`. Both examples display the same text.

Run `cargo run`. You should see the note inside a border, with each text child
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
