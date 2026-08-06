# Leptatui Examples

Runnable examples for the public `leptatui` crate live in this directory.
Each example includes responsive stylesheet rules for terminals at or below 60
columns, which is useful when running over SSH from a phone.

## Hello World

Run the Hello World example:

```sh
cargo run --example hello_world
```

Press `q` to exit.

## Block Layout

Run the block flow and terminal box-model example:

```sh
cargo run --example block_layout
```

The example contrasts `Div` and `Block`, content-box and border-box sizing,
physical spacing, percentage widths, and cumulative rounding across three
fractional children. Resize to 60 columns or fewer to stack the sizing panels.
Press `q` to exit.

## Responsive Flex

Run the responsive navigation, content, and sidebar example:

```sh
cargo run --example responsive_flex
```

The wide layout keeps navigation controls in a row and places a growing
content region beside a fixed-basis sidebar. Resize the terminal to 60 columns
or fewer to stack the navigation and workspace vertically. Press `q` to exit.

## Responsive Grid

Run the responsive dashboard example:

```sh
cargo run --example responsive_grid
```

The wide layout uses repeated fractional columns, a heading that spans the
explicit grid, intrinsic rows, and gapped summary and activity panels. Resize
the terminal to 60 columns or fewer to replace the two-column template with a
single stacked column. Press `q` to exit.

## Nested Overflow

Run the nested two-axis overflow example:

```sh
cargo run --example nested_overflow
```

The outer pane uses hidden horizontal overflow and automatic vertical
overflow. Its child pane uses automatic horizontal overflow and a forced
vertical scrollbar. Use `j`/`k` or Page Up/Page Down to scroll vertically.
Point at either pane and use vertical or horizontal wheel events to target it;
wheel scrolling bubbles to the outer pane when the inner pane reaches a
boundary. Press `q` to exit.

## Positioning Showcase

Run the positioning and stacking example:

```sh
cargo run --example positioning_showcase
```

The scrollport contains static, relative, absolute, and sticky boxes plus a
negative stacking layer. A fixed card remains attached to the terminal
viewport. Use `j`/`k` or Page Up/Page Down to scroll, verify that sticky and
fixed cards use different containing blocks, and observe the signed z-index
overlap. Press `q` to exit.

## Counter

Run the styled counter example:

```sh
cargo run --example counter
```

Press `+`/`=` to increment, `-` to decrement, `r` to reset, and `q` to quit.

## Controlled Form

Run the controlled form example:

```sh
cargo run --example controlled_form
```

Use Tab and Shift+Tab to move focus between the input, text area, and submit
button. Editable controls start in Vim normal mode; press `i`, `a`, `I`,
or `A` to enter insert mode. Press Esc while a form field is in normal mode to
cancel the form. Input fields submit with Enter. Text areas insert a newline in
insert mode with Enter and submit with Ctrl+Enter. Press `q` to quit.

## Standard Library Showcase

Run the standard component showcase:

```sh
cargo run --example standard_library_showcase
```

The showcase combines controlled `Input` and `TextArea` fields, `Form`
submit/cancel behavior, a standalone `Link`, supported mini-Vim editing, image
fallback text, and a progress bar in one app. Use Tab and Shift+Tab to move
focus. Press Enter or Space to activate links and buttons. Press `i`, `a`,
`I`, or `A` to enter insert mode; Esc or `jk` returns to normal mode; `v` and
`V` select text; `x`, `d`, `y`, `p`, `u`, and Ctrl+R edit in normal or visual
mode. Use Enter to submit inputs, Ctrl+Enter to submit text areas, activate
Advance or Reset to update progress, and press `q` to quit. The `Image` view
renders terminal graphics automatically when the terminal supports them and
otherwise displays deterministic fallback text.

## Default Styles Showcase

Run the complete default-style review app:

```sh
cargo run --example default_styles_showcase
```

The showcase places every visible built-in view except `Markdown` on one
scrollable page. Its stylesheet controls layout and sizing only, leaving each
view's default colors, borders, modifiers, focus state, active route state, and
editing-mode state unchanged. Use Tab and Shift+Tab to move focus, press `i` on
an input or text area to inspect the yellow insert state, press `v` or `V` to
inspect the magenta visual states, and press Esc to return to normal mode.
Activate either router anchor to switch the active style, use the buttons to
update the progress bar, and activate the standalone blue link to inspect its
magenta visited state. Scroll with the keyboard or mouse wheel, and press `q` to
quit.

## Document Showcase

Run the semantic document component showcase:

```sh
cargo run --example document_showcase
```

The showcase combines all six heading levels, paragraphs, nested ordered and
unordered lists, a responsive table with aligned cells, and a
syntax-highlighted code block with line numbers. Scroll with `j`/`k`, Page Up
and Page Down, `gg`, or `G`. Press `q` to quit.

Semantic views can be assembled with builders. Ordered-list starts, table-cell
alignment, and code presentation are configured on the concrete builder return
types:

```rust
use leptatui::prelude::*;

let document = div((
    h2("Nested list"),
    ordered_list([list_item((
        paragraph("Parent item"),
        unordered_list([list_item([paragraph("Nested item")])]),
    ))])
    .start(3),
    table([
        table_head([table_row([
            table_cell("Component"),
            table_cell("Layout").alignment(CellAlignment::Center),
        ])]),
        table_body([table_row([
            table_cell("Table"),
            table_cell("Responsive").alignment(CellAlignment::Right),
        ])]),
    ]),
    code_block("fn main() {}")
        .language("rust")
        .line_numbers(true),
));
```

The showcase uses the equivalent nested tags and semantic stylesheet
selectors:

```rust
use leptatui::prelude::*;

stylesheet! {
    H2 => { fg: Color::LightBlue }
    OrderedList => { fg: Color::LightCyan }
    TableHead => { fg: Color::LightCyan }
    CodeBlock => { fg: Color::LightBlue }
}

let document = view! {
    <Div>
        <OrderedList start=3>
            <ListItem>
                <Paragraph>"Parent item"</Paragraph>
                <UnorderedList>
                    <ListItem><Paragraph>"Nested item"</Paragraph></ListItem>
                </UnorderedList>
            </ListItem>
        </OrderedList>
        <CodeBlock
            language="rust"
            line_numbers=true
        >"fn main() {}"</CodeBlock>
    </Div>
};
```

All document content, including code, wraps to the terminal width instead of
scrolling horizontally. Syntax highlighting uses the terminal's ANSI palette
with `DarkGray` as its background, unknown code languages render as plain
source text, and table cells contain inline text rather than nested block views
in v1.

## Markdown Reader

Run the full-screen reader with its bundled showcase:

```sh
cargo run --example markdown_reader
```

Pass a path to read any local UTF-8 Markdown file instead:

```sh
cargo run --example markdown_reader -- README.md
```

The bundled fixture demonstrates headings, inline formatting, navigable links,
nested lists, aligned tables, readable fallback blocks, and known- and
unknown-language code fences:

```sh
cargo run --example markdown_reader -- crates/leptatui/examples/assets/markdown_showcase.md
```

The reader constructs its document from `<Markdown src={path} />` before
terminal startup. Unreadable paths and invalid UTF-8 open the reader with a
path-aware fallback paragraph. Scroll with the arrow keys or `j`/`k`, Page Up
and Page Down or Ctrl+U/Ctrl+D, `gg`, `G`, or the mouse wheel. Use Tab and
Shift+Tab or pointer movement to focus links, then Enter, Space, or left click
to open the focused target. Local Markdown files and heading fragments open
inside the reader; press Shift+H and Shift+L to move backward and forward
through page history. URLs and other local files use the system handler. Press
`q` to quit.

Use `markdown` and `markdown_with_options` when the source is already in
memory:

```rust
use leptatui::prelude::*;

let source = "# Reader\n\n```rust\nfn main() {}\n```";
let default_document = markdown(source);
let configured_document = markdown_with_options(
    source,
    MarkdownOptions::default().line_numbers(true),
);
```

Use the file functions or `Markdown` tag when the input is a local UTF-8 path:

```rust
use leptatui::prelude::*;

let default_file = markdown_file("README.md");
let configured_file = markdown_file_with_options(
    "README.md",
    MarkdownOptions::default().line_numbers(true),
);
let tagged_file = view! {
    <Markdown src="README.md" line_numbers=true />
};
```

Markdown support targets CommonMark plus tables. Optional GFM extensions such
as task lists, strikethrough, and footnotes are deferred. Markdown links retain
their labels and are keyboard navigable. Local targets resolve relative to the
source file's directory. Images render as descriptive text, and neither local
nor remote image targets are fetched.

## Error Handling

Run the fallible component and panic-cleanup showcase:

```sh
cargo run --example error_handling
```

Use `e` to propagate the real I/O error produced by reading the intentionally
absent `examples/demo-data.json` file, `c` for a replacement `view_error!`
message, and `s` for a source-preserving contextual error. On the error screen,
use `Esc` or `b` to return to the previous route and `q` to quit. Use `p` to
open the panic page, then activate its button to verify that the normal terminal
is restored before Rust prints the panic diagnostic. Press `h` to return Home.

## Theme Switcher

Run the context-backed light/dark theme example:

```sh
cargo run --example theme_switcher
```

Use Tab and Shift+Tab to move focus. Activate Toggle theme with Enter or Space
to switch the active theme variables at runtime.

## Multi-Page Demo

Run the route-driven Home, Counter, and Settings demo:

```sh
cargo run --example multi_page_demo
```

Use `h`, `c`, and `s` to switch pages. The Counter page supports `+`, `-`, and
`r`; the Settings page supports `t` for theme switching. Press `q` to quit.
Counter and theme settings are shared through root context, so changes persist
while moving between pages.

## Style Cascade Showcase

Run the style cascade mechanics showcase:

```sh
cargo run --example style_cascade_showcase
```

This demonstrates CSS-like specificity, source order, descendant selectors,
component boundaries, inline styles, `!important`, inheritance, and focus
styling. Use Tab and Shift+Tab to move focus. Activate buttons with Enter or
Space, and press `q` to quit.

## Stylesheet Imports

Run the imported variables and mixins example:

```sh
cargo run --example stylesheet_imports
```

This demonstrates `@use`, namespaced variables, imported mixins, and local
mixin composition. Use Tab and Shift+Tab to move focus. Activate buttons with
Enter or Space, and press `q` to quit.

## Async Redraw

Run the async redraw example:

```sh
cargo run --example async_redraw
```

The initial resource completion should redraw after its delay without key
input. Use `r` to reload, `a` to dispatch the async action, and `q` to quit.

## Async CRUD

Run the async CRUD-style mock API demo:

```sh
cargo run --example async_crud
```

The demo loads a mock ticket list, dispatches create/update actions, refetches
after successful mutations, and renders pending, success, and error states.
Use `n` to create, `u` to update the first ticket, `r` to reload, `l` to fail
the next list request, `e` to fail a mutation, and `q` to quit.

## External Editor

Run the external editor hook example with its in-memory Markdown draft:

```sh
cargo run --example external_editor
```

Supply an optional path to enable the file-editing action as well:

```sh
cargo run --example external_editor -- README.md
```

Press `t` to round-trip the displayed `RwSignal<String>` through a temporary
`.md` file, `f` to edit the optional path, `c` to clear the editor status, and
`q` to quit. The status line reactively shows the shared `EditorStatus` moving
from pending to success or failure. The editor comes from `VISUAL`, then
`EDITOR`, and finally the `vi` fallback.

## File System Showcase

Run the complete root-scoped filesystem operation laboratory:

```sh
cargo run --example file_system_showcase
```

The showcase confines every operation to a process-specific directory below
the platform temporary directory. Press `n` or Enter to execute one unlocked
step. The next step remains locked until the current asynchronous operation
produces its expected result, so `copy_file()` always completes before
`rename()` can start. Completed results remain visible and `b` moves backward
through them without undoing filesystem changes.

| Function | Successful tour | What it does |
| --- | ---: | --- |
| `use_file_system_with_options(root, options)` | Setup | Expands a leading `~`, recursively creates the example root, and returns the component-local handle. |
| `root()` | Setup | Returns the canonical boundary displayed above the walkthrough. |
| `create_dir(path)` | 1 | Recursively creates `walkthrough/nested/tree`. |
| `write_file(path, contents)` | 2 | Creates `demo.txt` and writes its initial bytes. |
| `append_file(path, contents)` | 3 | Adds bytes to the end of `demo.txt`. |
| `resolve_path(path)` | 4 | Canonicalizes `demo.txt` and enforces containment. |
| `get_metadata(path)` | 5 | Reports the file's kind, size, permissions, and timestamps. |
| `read_dir(path)` | 6 | Lists safe contained entries in deterministic order. |
| `read_file_as_bytes(path)` | 7 | Reads the complete file without decoding it. |
| `read_file_as_string(path)` | 8 | Reads and UTF-8 decodes the complete file. |
| `write_and_replace_file(path, contents)` | 9 | Writes a sibling temporary file, then replaces `demo.txt`. |
| `copy_file(source, destination)` | 10 | Copies `demo.txt` to `copy.txt` without overwriting. |
| `rename(source, destination)` | 11 | Moves the completed `copy.txt` operation to `moved.txt`; it cannot run early. |
| `delete_file(path)` | 12 | Removes `moved.txt`. |
| `delete_dir(path)` | 13 | Recursively removes the walkthrough tree while preserving the scoped root. |

Each method immediately starts and returns a `FileOperation<T>`. Retain that
handle to observe pending, result, and version state or retry the same captured
arguments with `dispatch(())`. The walkthrough uses that retry behavior when an
unexpected result occurs: press `r` to rerun the same captured operation.

After the successful tour, press `f` for an optional ordered failure tour. Its
visible setup and cleanup steps demonstrate invalid UTF-8, a missing file,
parent traversal, `~` expansion outside the root, a non-overwriting copy
conflict, and protected-root deletion. An expected error counts as a passed
step; any different result stops the tour for inspection and retry.

Use `n` or Enter to run or advance, `b` to inspect previous results, `r` to
retry an unexpected failure, Shift+`R` to clean the walkthrough subtree and
restart, `f` to enter the failure tour, and `q` to quit. Reset never deletes the
process-specific scoped root.
