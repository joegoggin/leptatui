# Async notes app

Complete checkpoint for [chapter 9](../guide/09-async-data.md).
Use the Cargo manifest from [Getting started](../guide/01-getting-started.md).
Replace the entire `src/main.rs` with the following code, then run `cargo run`.
Ctrl+Q quits from every page.

```rust
use leptatui::prelude::*;

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
fn NoteEditor() -> impl IntoView {
    let Draft {
        title,
        body,
        status,
    } = expect_context::<Draft>();
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

#[component]
fn NotePreview() -> impl IntoView {
    let Draft { title, body, .. } = expect_context::<Draft>();
    view! {
        <Div>
            <H2>{title}</H2>
            <Paragraph>{body}</Paragraph>
        </Div>
    }
}

#[component]
fn NotFound() -> impl IntoView {
    view! { <Text>"Page not found. Choose Edit or Preview."</Text> }
}

#[component]
fn DataToolbar() -> impl IntoView {
    let Draft { body, status, .. } = expect_context::<Draft>();
    let loaded = Resource::new(
        || (),
        |_| async {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            Ok::<_, String>(String::from("A note from the mock service"))
        },
    );
    let save = Action::new(|body: &String| {
        let body = body.clone();
        async move {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            if body.trim().is_empty() {
                Err(String::from("Write something before saving"))
            } else {
                Ok(format!("Accepted {} bytes", body.len()))
            }
        }
    });
    let reload = loaded.clone();
    let apply = loaded.clone();
    let save_button = save.clone();
    view! {
        <Div>
            <Div class="actions">
                <Button on_press=move || {
                    reload.refetch();
                    AppControl::Continue
                }>"Reload"</Button>
                <Button on_press=move || {
                    if let Some(Ok(value)) = apply.get_untracked() {
                        body.set(value);
                        status.set(String::from("Loaded note applied"));
                    }
                    AppControl::Continue
                }>"Use loaded note"</Button>
                <Button on_press=move || {
                    save_button.dispatch(body.get_untracked());
                    AppControl::Continue
                }>"Save body"</Button>
            </Div>
            <Text>{move || {
                if loaded.is_loading() {
                    String::from("Loading...")
                } else {
                    match loaded.get() {
                        None => String::from("Not loaded"),
                        Some(Ok(_)) => String::from("Loaded; choose Use loaded note to apply"),
                        Some(Err(error)) => format!("Load failed: {error}"),
                    }
                }
            }}</Text>
            <Text>{move || {
                if save.is_pending() {
                    String::from("Saving...")
                } else {
                    match save.value().get() {
                        None => String::from("No save yet"),
                        Some(Ok(message)) => message,
                        Some(Err(error)) => format!("Save failed: {error}"),
                    }
                }
            }}</Text>
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
    provide_context(Draft {
        title,
        body,
        status,
    });
    stylesheet! {
        .notes-shell => {
            padding: TuiSpacing::uniform(1)
            .actions => {
                display: Display::Flex,
                gap: Axes::all(Length::cells(1.0))
                @media (max-width: 60) {
                    flex_direction: FlexDirection::Column
                }
            }
        }
        H1 => { fg: Color::LightCyan }
        A:active => { fg: Color::LightGreen }
        Button:focus => { fg: Color::Yellow }
    }
    view! {
        <Router initial_path="/edit">
            <Div class="notes-shell">
                <NoteHeading text="My notes" />
                <Div class="actions">
                    <A href="/edit">"Edit"</A>
                    <A href="/preview">"Preview"</A>
                </Div>
                <DataToolbar />
                <Routes fallback=NotFound>
                    <Route path="/edit" view=NoteEditor />
                    <Route path="/preview" view=NotePreview />
                </Routes>
            </Div>
        </Router>
    }
}

#[tokio::main]
async fn main() -> leptatui::app::Result<()> {
    App::new(view! { <NotesApp /> }).run().await
}
```

## Check the result

Wait for loading without pressing a key. Apply the loaded note, save, and observe the pending and accepted states. Saving an empty body reports a local error. The mock service does not persist data.

[Return to chapter 9](../guide/09-async-data.md) · [Documentation index](../README.md)
