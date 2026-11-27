# 4. Editing notes

Continue from [chapter 3: State and interaction](03-state-and-interaction.md).

The counter showed how a button can change a signal. Now you'll use
signals to hold a note's title and body while you type.

In `src/app.rs`, after the quit handler in `NotesApp`, replace `count` with:

```rust
let title = RwSignal::new(String::from("First note"));
let body = RwSignal::new(String::from("Learn to build a terminal application."));
let status = RwSignal::new(String::from("Editing"));
```

Replace the view expression with:

```rust
view! {
    <Div>
        <Text>"My notes"</Text>
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
        <Text>"Tab focuses. i edits. Esc returns to normal mode. Enter submits in normal mode. Ctrl+Q quits."</Text>
    </Div>
}
```

Look at `<Input value=title ... />`. When you type in the title field,
Leptatui writes the edited text to the `title` signal. The body field works
the same way through `value=body`. Passing the signal as `value` is enough
to keep the field and the draft in sync.

This form uses `on_input` for a second job. After the field writes the new
text to its signal, Leptatui calls `on_input` and the callback sets the status
back to `Editing`. The callback takes no arguments. If it needs the new text,
it can read it with `title.get()` or `body.get()`.

The binding also works in the other direction. If your code calls
`title.set(...)`, `view!` tracks that change and updates the existing title
field. The field keeps its focus, cursor, selection, undo history, and editing
mode. This kind of update leaves `on_input` idle, so it also leaves the status
message alone unless your code changes it.

Like the counter callbacks in chapter 3, these callbacks return `()`, so
the app keeps running. A callback can return `AppControl::Exit` to quit.

## Try editing

Run `cargo run` to try the form.

1. Press Tab to focus the title. Controls start in Vim-style normal mode.
2. Press `i`, type a title, then Esc to return to normal mode.
3. Press Tab to reach the body, then `i`. Enter inserts a new line here.
4. Press Esc, then Enter to submit from the body. Ctrl+Enter submits from the
   body in normal or insert mode; Enter submits from the title in either mode.
5. Esc from normal mode invokes the form's cancel callback.

Submission currently updates a status message; it does not write a file.
Cancel also keeps the draft because that is what our callback implements.
Restore a saved snapshot explicitly if your application needs discard behavior.

The [component reference](../reference/components.md) explains additional
movement, selection, undo, redo, and clipboard commands. Run
`cargo run --example controlled_form` for a separate form demonstration.

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

    let title = RwSignal::new(String::from("First note"));
    let body = RwSignal::new(String::from("Learn to build a terminal application."));
    let status = RwSignal::new(String::from("Editing"));

    view! {
        <Div>
            <Text>"My notes"</Text>
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

[Previous: State and interaction](03-state-and-interaction.md) · [Documentation index](../README.md) · [Next: Reusable components](05-reusable-components.md)
