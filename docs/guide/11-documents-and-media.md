# 11. Documents and media

Prerequisites: [Chapter 10: Files and editors](10-files-and-editors.md).

Continue from chapter 10 and extend the preview with Markdown formatting.
The complete app below keeps file loading, saving, and all existing routes.
The later document and media examples are optional experiments.

## Preview Markdown

In `NotePreview`, replace the paragraph displaying the body with a tracked
Markdown rendering helper inside `view!`:

```rust
{move || markdown_with_options(
    body.get(),
    MarkdownOptions::default().line_numbers(true),
)}
```

Type `# Heading` and a fenced Rust code block into the editor, then navigate
to Preview. You should see semantic headings and highlighted source. This
renders the draft in memory; it does not read or write a file. The `Markdown`
tag loads files, so this in-memory preview needs `markdown_with_options`;
there is currently no equivalent tag for rendering a string.

For synchronous UTF-8 file loading, use `markdown_file` or
`markdown_file_with_options`. `markdown_source_with_options` supplies a source
path identity to in-memory text so relative document links have a base. As an optional experiment, replace `NotePreview`'s final `view!` expression
with the following to load a path asynchronously. Save a note through the
file toolbar first so the file exists; restore the in-memory Markdown preview
before chapter 12:

```rust
view! { <Markdown src="notes-data/note.md" editable=true line_numbers=true /> }
```

`editable=true` adds `e` to edit the original source and `r` to reload it.
Successful editing also reloads it. Reload resets this reader's local history,
focus, and scroll state. Read, editor, and reload failures use the standard
error screen.

File-backed Markdown links to `.md`/`.markdown` documents open in the reader;
fragments scroll to heading anchors. Shift+H/L navigates cached page history.
URLs and other files use the system handler. Standalone links and in-memory
Markdown also use external opening behavior. Markdown images become descriptive
text without fetching their targets. CommonMark plus tables is supported;
optional GFM extensions remain deferred.

## Optional: Build documents directly

Inside a component body, use `view!` to create a semantic document without
parsing Markdown:

```rust
let document = view! {
    <Div>
        <H1>"Notes"</H1>
        <Paragraph>"Things to learn"</Paragraph>
        <OrderedList start=1>
            <ListItem>
                <Paragraph>"Understand signals"</Paragraph>
                <UnorderedList>
                    <ListItem><Paragraph>"Try a tracked closure"</Paragraph></ListItem>
                </UnorderedList>
            </ListItem>
        </OrderedList>
        <Table>
            <TableHead>
                <TableRow><TableCell>"Topic"</TableCell><TableCell>"Status"</TableCell></TableRow>
            </TableHead>
            <TableBody>
                <TableRow>
                    <TableCell>"Editing"</TableCell>
                    <TableCell alignment={CellAlignment::Center}>"Ready"</TableCell>
                </TableRow>
            </TableBody>
        </Table>
        <CodeBlock language="rust" line_numbers=true>"let count = 1;"</CodeBlock>
    </Div>
};
```

To try this experiment, replace `NotePreview`'s body with the binding above
and return `document`. Restore the Markdown preview before chapter 12.

Textual views accept owned Ratatui rich text (`RichText`) for
styled spans. Code wraps to the terminal width, unknown languages stay plain,
and table cells contain inline text rather than nested block views.

## Optional: Images, links, and progress

To try these views, insert them as children of `NotePreview`'s `Div`.
Remove them before continuing to chapter 12.

Use `<Link href="https://example.com">"Project site"</Link>` for a URL or
system-opened file. Links participate in normal focus and activation and match
`:visited` after a successful visit.

For a local image, supply your own `photo.png` in the notes project's working
directory and insert `<Image src="photo.png" alt="A photo" />`. The core app
does not require an image asset. `ImageSource` represents its path-backed source. Supported terminals render graphics; other
terminals display deterministic fallback text. This differs from an image
inside Markdown, which always uses descriptive text.

Use `<ProgressBar value=0.5 label="Saving" />` to show progress. Values clamp to
`0.0..=1.0`; use `value=progress.get()` to track a progress signal automatically.

Run `document_showcase`, `markdown_reader`, or `standard_library_showcase`
with `cargo run --example NAME`. The [component reference](../reference/components.md)
details document structure, defaults, links, and fallback behavior.

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

#[component]
fn NoteHeading(#[prop(into)] text: String) -> impl IntoView {
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

    stylesheet! {
        H1 { fg: Color::LightCyan; }
        A:active { fg: Color::LightGreen; }
        Button:focus { fg: Color::Yellow; }

        .notes-shell {
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

[Previous: Files and editors](10-files-and-editors.md) · [Documentation index](../README.md) · [Next: Themes and reusable styles](12-themes-and-reusable-styles.md)
