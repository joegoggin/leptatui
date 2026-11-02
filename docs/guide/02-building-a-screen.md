# 2. Building a screen

Prerequisites: [Chapter 1: Getting started](01-getting-started.md).

Build a static note and learn how the view tree describes its structure.
Keep the imports, quit handler, and `main` from chapter 1. Replace only the
`view!` expression in `NotesApp` with:

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

## The builder alternative

You can return the same structure using ordinary Rust functions. Replace the
whole `view!` expression above with:

```rust
block(div((
    text("My notes"),
    text("First note"),
    paragraph("Learn to build a terminal application."),
    text("Ctrl+Q quits"),
)))
```

A tuple allows different child view types in one container. Builders keep their
concrete types so you can call their configuration methods; both approaches
ultimately implement `IntoView`. Use markup for the remaining tutorial.

Run the app. You should see the note inside a border, with each text child
below the previous one. Resize the terminal to see paragraph wrapping.

The [component reference](../reference/components.md) lists the available
views. We will introduce them as the application needs them.

[Previous: Getting started](01-getting-started.md) · [Documentation index](../README.md) · [Next: State and interaction](03-state-and-interaction.md)
