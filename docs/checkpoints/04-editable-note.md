# Editable note

Complete checkpoint for [chapter 4](../guide/04-editing-notes.md).
Use the Cargo manifest from [Getting started](../guide/01-getting-started.md).
Replace the entire `src/main.rs` with the following code, then run `cargo run`.
Ctrl+Q quits from every page.

```rust
use leptatui::prelude::*;

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
    view! {
        <Div>
            <Text>"My notes"</Text>
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
            <Text>{status}</Text>
            <Text>"Tab focuses. i edits. Esc returns to normal mode. Ctrl+Q quits."</Text>
        </Div>
    }
}

#[tokio::main]
async fn main() -> leptatui::app::Result<()> {
    App::new(view! { <NotesApp /> }).run().await
}
```

## Check the result

Tab to the title, press i, and edit. Esc returns to normal mode. Ctrl+Enter submits from the body. Submit/cancel only change the status; the draft stays in memory.

[Return to chapter 4](../guide/04-editing-notes.md) · [Documentation index](../README.md)
