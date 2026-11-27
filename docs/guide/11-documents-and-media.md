# 11. Documents and media

Continue from [chapter 10: Files and editors](10-files-and-editors.md).

Preview already shows the draft text. Let's make it render headings
and code blocks as you would see them in a Markdown reader.

## Preview Markdown

In `NotePreview` in `src/pages/preview/page.rs`, replace the paragraph displaying the body with a tracked
Markdown rendering helper inside `view!`:

```rust
{move || markdown_with_options(
    body.get(),
    MarkdownOptions::default().line_numbers(true),
)}
```

Run `cargo run` and enter this sample in the body. Tab to Preview and press Enter.

````markdown
# Heading

```rust
let count = 1;
```
````

You should see semantic headings and highlighted source. This
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

## Complete files for this chapter

Create or replace only the files listed below; retain all other files from
the previous chapter. Keep the Cargo manifest from [chapter 1](01-getting-started.md).

Run `cargo run` in your application project. Ctrl+Q quits.

### `src/pages/preview/page.rs`

```rust
use leptatui::prelude::*;

use crate::draft::Draft;

#[component]
pub(crate) fn NotePreview() -> impl IntoView {
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
```

[Previous: Files and editors](10-files-and-editors.md) · [Documentation index](../README.md) · [Next: Themes and reusable styles](12-themes-and-reusable-styles.md)
