# 12. Themes and reusable styles

Prerequisites: [Chapter 11: Documents and media](11-documents-and-media.md).

Make the app's appearance reusable. Keep literal palette colors in a theme
model and use named variables in component styles.

Inside `NotesApp`, after providing `Draft` and before its stylesheet, add:

```rust
let light = RwSignal::new(false);
let theme = RwSignal::new(
    ThemeVariables::new()
        .color("text", Color::White)
        .color("surface", Color::Black),
);
provide_context(theme.read_only());
```

In the existing shell stylesheet, remove `H1 { fg: Color::LightCyan; }`;
we will style the heading through a reusable mixin below. Add these declarations
inside the existing `.notes-shell` rule, keeping its layout rules:

```rust
fg: theme_color("text");
bg: theme_color("surface");
```

Add the following button after Preview in the navigation `Div`:

```rust
<Button on_press=move || {
    light.update(|value| *value = !*value);
    let (text, surface) = if light.get_untracked() {
        (Color::Black, Color::White)
    } else {
        (Color::White, Color::Black)
    };
    theme.set(ThemeVariables::new().color("text", text).color("surface", surface));
}>"Toggle theme"</Button>
```

Styles resolve variables while rendering, so descendants repaint without
changing their component code. For fixed themes, provide a plain
`ThemeVariables` instead of its read signal. Run the app, toggle twice, and
verify that the text and surface colors switch back while your draft remains.
The separate library example is `cargo run --example theme_switcher`.

## Variables, mixins, and imports

Local `$variables` avoid repeating values; `@mixin` defines reusable
declarations. Helpers returning only variables and mixins produce a
`StyleModule`, which another stylesheet imports with `@use`. Add `note_styles`
above `NoteHeading`, then replace `NoteHeading` with the definition below.
Keep its `text` prop so existing callers continue to work:

```rust
fn note_styles() -> StyleModule {
    stylesheet! {
        $accent: Color::LightCyan;
        @mixin heading {
            fg: $accent;
            modifier: Modifier::BOLD;
        }
    }
}

#[component]
fn NoteHeading(#[prop(into)] text: String) -> impl IntoView {
    stylesheet! {
        @use note_styles as notes;
        H1 { @include notes.heading; }
    }
    view! { <H1>{text}</H1> }
}
```

Nested rules express descendant selectors. `&:focus`, `&:active`, `&:insert`,
`&:visual`, and `&:visited` combine supported states with the containing
selector. Beneath a class, `&__element` and `&--modifier` concatenate class
names, useful for component naming conventions. Specificity and source order
choose between rules; inline styles override normal stylesheet rules, while
stylesheet `!important` overrides normal inline declarations. Inheritable
values flow through the tree; do not assume every layout property inherits.

As components grow, place a component and its style helper together in a
module directory. A helper in `style.rs` should invoke `stylesheet!` during
component setup and end it with a semicolon when returning `()`. Routed pages
can use the same pattern. The [stylesheet reference](../reference/stylesheets.md)
contains the module layout and exact cascade examples.

Run `stylesheet_imports`, `style_cascade_showcase`, or
`default_styles_showcase` with `cargo run --example NAME`.

## Complete code

Replace `src/main.rs` with the following. Use the Cargo manifest from
[chapter 1](01-getting-started.md) and run `cargo run` in your application
project. Ctrl+Q quits.

```rust
use leptatui::prelude::*;

#[derive(Clone, Copy)]
struct Draft {
    title: RwSignal<String>,
    body: RwSignal<String>,
    status: RwSignal<String>,
}

fn note_styles() -> StyleModule {
    stylesheet! {
        $accent: Color::LightCyan;
        @mixin heading {
            fg: $accent;
            modifier: Modifier::BOLD;
        }
    }
}

#[component]
fn NoteHeading(#[prop(into)] text: String) -> impl IntoView {
    stylesheet! {
        @use note_styles as notes;
        H1 { @include notes.heading; }
    }
    view! { <H1>{text}</H1> }
}

#[component]
fn NoteEditor() -> impl IntoView {
    let Draft { title, body, status } = expect_context::<Draft>();

    view! {
        <Div>
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

#[component]
fn NotePreview() -> impl IntoView {
    let Draft { title, body, .. } = expect_context::<Draft>();
    view! {
        <Div>
            <H2>{title}</H2>
            {move || markdown_with_options(
                body.get(),
                MarkdownOptions::default().line_numbers(true),
            )}
        </Div>
    }
}

#[component]
fn NotFound() -> impl IntoView {
    view! { <Text>"Page not found. Choose Edit or Preview."</Text> }
}

#[derive(Clone, RouteParams)]
struct NoteParams {
    id: usize,
}

#[derive(Clone, QueryParams)]
struct NoteQuery {
    line: Option<usize>,
}

#[component]
fn NoteDetails() -> ViewResult<impl IntoView> {
    let params = use_params::<NoteParams>()?;
    let query = use_query::<NoteQuery>()?;
    view! { <Text>{format!("Note {}, line {:?}", params.id, query.line)}</Text> }
}

#[component]
fn RequiredFile() -> ViewResult<impl IntoView> {
    let source = match std::fs::read_to_string("note.md") {
        Ok(source) => source,
        Err(error) => view_error!(error => "Could not load note.md"),
    };
    view! { <Paragraph>{source}</Paragraph> }
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
            <Div class="notes-shell__actions">
                        <Button on_press=move || {
                            latest_read.set(Some(read_files.read_file_as_string("note.md")));
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
                }>"Use loaded note"</Button>
                <Button on_press=move || {
                    latest_write.set(Some(files.write_and_replace_file(
                        "note.md", body.get_untracked(),
                    )));
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

    provide_context(Draft { title, body, status });

    let light = RwSignal::new(false);
    let theme = RwSignal::new(
        ThemeVariables::new()
            .color("text", Color::White)
            .color("surface", Color::Black),
    );
    provide_context(theme.read_only());
    stylesheet! {
        A:active { fg: Color::LightGreen; }
        Button:focus { fg: Color::Yellow; }

        .notes-shell {
            fg: theme_color("text");
            bg: theme_color("surface");
            padding: TuiSpacing::uniform(1);
            &__actions {
                display: Display::Flex;
                gap: Axes::all(Length::cells(1.0));
                @media (max-width: 60) {
                    flex_direction: FlexDirection::Column;
                }
            }
        }
    }
    view! {
        <Router initial_path="/edit">
            <Div class="notes-shell">
                <NoteHeading text="My notes" />
                <Div class="notes-shell__actions">
                    <A href="/edit">"Edit"</A>
                    <A href="/preview">"Preview"</A>
                <Button on_press=move || {
                    light.update(|value| *value = !*value);
                    let (text, surface) = if light.get_untracked() {
                        (Color::Black, Color::White)
                    } else {
                        (Color::White, Color::Black)
                    };
                    theme.set(ThemeVariables::new().color("text", text).color("surface", surface));
                }>"Toggle theme"</Button>
                    <A href="/notes/1?line=3">"Note details"</A>
                    <A href="/notes/abc">"Invalid note"</A>
                    <A href="/required-file">"Read note.md"</A>
                </Div>
                <FileToolbar />
                <Routes fallback=NotFound>
                    <Route path="/edit" view=NoteEditor />
                    <Route path="/preview" view=NotePreview />
                    <Route path="/notes/:id" view=NoteDetails />
                    <Route path="/required-file" view=RequiredFile />
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

[Previous: Documents and media](11-documents-and-media.md) · [Documentation index](../README.md) · [Next: Extending Leptatui](13-extending-leptatui.md)
