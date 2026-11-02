# 11. Documents and media

Prerequisites: [Chapter 10: Files and editors](10-files-and-editors.md).

Extend the preview with document formatting. The examples here are focused
additions; the file checkpoint remains the core app.

## Preview Markdown

In `NotePreview`, replace the paragraph displaying the body with a tracked
Markdown builder:

```rust
{move || markdown_with_options(
    body.get(),
    MarkdownOptions::default().line_numbers(true),
)}
```

Type `# Heading` and a fenced Rust code block into the editor, then navigate
to Preview. You should see semantic headings and highlighted source. This
renders the draft in memory; it does not read or write a file.

For synchronous UTF-8 file loading, use `markdown_file` or
`markdown_file_with_options`. `markdown_source_with_options` supplies a source
path identity to in-memory text so relative document links have a base. To
load a path asynchronously, return a view containing:

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

## Build documents directly

Inside a component body, this builder expression creates a small semantic
document without parsing Markdown:

```rust
let document = div((
    h1("Notes"),
    paragraph("Things to learn"),
    ordered_list([list_item((
        paragraph("Understand signals"),
        unordered_list([list_item([paragraph("Try a tracked closure")])]),
    ))]).start(1),
    table([
        table_head([table_row([table_cell("Topic"), table_cell("Status")])]),
        table_body([table_row([
            table_cell("Editing"),
            table_cell("Ready").alignment(CellAlignment::Center),
        ])]),
    ]),
    code_block("let count = 1;").language("rust").line_numbers(true),
));
```

Return `document` or insert it as `{document}`. Headings `h1`–`h6`, paragraphs,
list sections/items, table sections/rows/cells, and code blocks have equivalent
PascalCase tags. Textual views accept owned Ratatui rich text (`RichText`) for
styled spans. Code wraps to the terminal width, unknown languages stay plain,
and table cells contain inline text rather than nested block views.

## Images, links, and progress

Use `<Link href="https://example.com">"Project site"</Link>` for a URL or
system-opened file. Links participate in normal focus and activation and match
`:visited` after a successful visit.

Use `<Image src="photo.png" alt="A photo" />` for a local image. `ImageSource`
represents its path-backed source. Supported terminals render graphics; other
terminals display deterministic fallback text. This differs from an image
inside Markdown, which always uses descriptive text.

Use `progress_bar(0.5).label("Saving")` or
`<ProgressBar value=0.5 label="Saving" />` to show progress. Values clamp to
`0.0..=1.0`; wrap a signal read in a dynamic closure to update the gauge.

Run `document_showcase`, `markdown_reader`, or `standard_library_showcase`
with `cargo run --example NAME`. The [component reference](../reference/components.md)
details document structure, defaults, links, and fallback behavior.

[Previous: Files and editors](10-files-and-editors.md) · [Documentation index](../README.md) · [Next: Themes and reusable styles](12-themes-and-reusable-styles.md)
