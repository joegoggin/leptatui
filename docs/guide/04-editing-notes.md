# 4. Editing notes

Prerequisites: [Chapter 3: State and interaction](03-state-and-interaction.md).

Turn the static note into an editable title and body. This chapter replaces
the revision demonstration; we now keep the actual note in signals.

After the quit handler, replace `revisions` with:

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
        {move || view! {
            <Form
                on_submit=move || {
                    status.set(String::from("Submitted in memory"));
                    AppControl::Continue
                }
                on_cancel=move || {
                    status.set(String::from("Canceled; draft kept"));
                    AppControl::Continue
                }
            >
                <Text>"Title"</Text>
                <Input value={title.get()} placeholder="Title"
                    on_input=move |next| {
                        title.set(next);
                        status.set(String::from("Editing"));
                        AppControl::Continue
                    }
                />
                <Text>"Body"</Text>
                <TextArea value={body.get()} placeholder="Write a note"
                    on_input=move |next| {
                        body.set(next);
                        status.set(String::from("Editing"));
                        AppControl::Continue
                    }
                />
            </Form>
        }}
        <Text>{status}</Text>
        <Text>"Tab focuses. i edits. Esc returns to normal mode. Ctrl+Q quits."</Text>
    </Div>
}
```

The controls are **controlled**: `value` comes from your state, and `on_input`
proposes a new string. Accept the change by storing it in the signal. Without
that update, the displayed value does not change. The tracked closure reads
the current values and rebuilds the form when they change. Reconciliation
preserves the controls' focus, cursor, and editing state across these rebuilds.

## Try editing

1. Press Tab to focus the title. Controls start in Vim-style normal mode.
2. Press `i`, type a title, then Esc to return to normal mode.
3. Press Tab to reach the body, then `i`. Enter inserts a new line here.
4. Ctrl+Enter submits from the body; Enter submits from the title.
5. Esc from normal mode invokes the form's cancel callback.

Submission currently updates a status message; it does not write a file.
Cancel also keeps the draft because that is what our callback implements.
Restore a saved snapshot explicitly if your application needs discard behavior.

[Complete editable-note checkpoint](../checkpoints/04-editable-note.md).
The [component reference](../reference/components.md) explains additional
movement, selection, undo, redo, and clipboard commands. Run
`cargo run --example controlled_form` for a separate form demonstration.

[Previous: State and interaction](03-state-and-interaction.md) · [Documentation index](../README.md) · [Next: Reusable components](05-reusable-components.md)
