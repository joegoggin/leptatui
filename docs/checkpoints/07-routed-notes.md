# Routed notes app

Complete checkpoint for [chapter 7](../guide/07-pages-and-navigation.md).
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

Edit the draft, navigate to Preview, then return to Edit. The shared draft should survive navigation. Resize below 60 columns to stack the navigation links.

[Return to chapter 7](../guide/07-pages-and-navigation.md) · [Documentation index](../README.md)
