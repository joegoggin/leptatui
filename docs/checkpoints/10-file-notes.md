# File-backed notes app

Complete checkpoint for [chapter 10](../guide/10-files-and-editors.md).
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
fn FileToolbar() -> ViewResult<impl IntoView> {
    let Draft { body, status, .. } = expect_context::<Draft>();
    let files =
        use_file_system_with_options("notes-data", FileSystemOptions::new().create_root(true))?;
    let latest_read = RwSignal::new(None::<FileOperation<String>>);
    let latest_write = RwSignal::new(None::<FileOperation<()>>);
    let read_files = files.clone();
    view! {
        <Div>
            <Div class="actions">
                <Button on_press=move || {
                    latest_read.set(Some(read_files.read_file_as_string("note.md")));
                    AppControl::Continue
                }>"Load body"</Button>
                <Button on_press=move || {
                    if let Some(read) = latest_read.get_untracked() {
                        read.value().with_untracked(|result| {
                            if let Some(Ok(value)) = result {
                                body.set(value.clone());
                                status.set(String::from("File applied to draft"));
                            }
                        });
                    }
                    AppControl::Continue
                }>"Use loaded note"</Button>
                <Button on_press=move || {
                    latest_write.set(Some(files.write_and_replace_file(
                        "note.md", body.get_untracked(),
                    )));
                    AppControl::Continue
                }>"Save body"</Button>
            </Div>
            <Text>{move || match latest_read.get() {
                None => String::from("No read yet"),
                Some(read) if read.is_pending() => String::from("Reading..."),
                Some(read) => read.value().with(|result| match result {
                    Some(Ok(_)) => String::from("Read complete; choose Use loaded note"),
                    Some(Err(error)) => format!("Read failed: {error}"),
                    None => String::from("Waiting for read"),
                }),
            }}</Text>
            <Text>{move || match latest_write.get() {
                None => String::from("No save yet"),
                Some(write) if write.is_pending() => String::from("Saving..."),
                Some(write) => write.value().with(|result| match result {
                    Some(Ok(())) => String::from("Saved notes-data/note.md"),
                    Some(Err(error)) => format!("Save failed: {error}"),
                    None => String::from("Waiting for save"),
                }),
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
                <FileToolbar />
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

Save a draft, load it, and apply the result. Restart and load again to verify persistence. The body is stored at notes-data/note.md relative to your working directory; the title is not persisted. Loading a missing file displays an error and keeps your draft.

[Return to chapter 10](../guide/10-files-and-editors.md) · [Documentation index](../README.md)
