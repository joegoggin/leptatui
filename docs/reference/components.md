# Standard components reference

Start with [Building a screen](../guide/02-building-a-screen.md), [Editing notes](../guide/04-editing-notes.md), and [Documents and media](../guide/11-documents-and-media.md).

## On this page

- [Standard Components](#standard-components)
- [Semantic Documents](#semantic-documents)
- [Markdown Readers](#markdown-readers)

## Standard Components

Leptatui's standard view set includes layout containers (`Block`, `Div`),
plain and semantic text, nested lists, responsive tables,
syntax-highlighted code blocks, CommonMark documents, buttons, controlled form
controls, images, and progress bars.
All are available through `leptatui::prelude::*` as builders and through
PascalCase `view!` tags.

`view!` tracks data expressions automatically, including text, attributes, and
custom component data props. Write `<Input value={title.get()} ... />` directly;
updates change the retained control and preserve its editing state. Signal
reads inside event handlers run only for events and do not subscribe rendering.
Values computed before `view!` remain snapshots, and plain Rust builders use
ordinary Rust evaluation semantics.

Custom components declare changing data as `Prop<T>` and read it with `.get()`
or `.with()` inside their view expressions. Component setup runs once. Use
`#[prop(callback)]` for retained event callbacks and `#[prop(static)]` for
intentional setup-only inputs such as a router's initial location.

`View` is an open, object-safe trait. Built-in builders retain concrete return
types such as `TextView`, `DivView`, `InputView`, and `TextAreaView`, so their
type-specific configuration remains available without pattern matching.
Containers accept homogeneous collections or heterogeneous tuples through
`IntoViews`. Braced `Vec<V>` expressions inside `Div`, `Form`, `ListItem`, or
component children render in source order without an extra layout box. Computed
collections update through a transparent reactive boundary.
Type erasure occurs only when children enter the tree as `AnyView` values.
Application-defined views can implement `View` directly,
optionally implement `StyledView` or `ContainerView`, and use
`RenderCtx::resolve_style` to participate in the normal stylesheet cascade.
Custom selector names are created with `ViewType::new("Name")`, and the same
name can be used as a `stylesheet!` type selector.

`Link` opens URLs and local paths with the operating system's configured
handler. Use `link("Project site", "https://example.com")` or
`<Link href="https://example.com">"Project site"</Link>` anywhere in a view.
Links are blue and underlined by default, then magenta and underlined after a
successful visit during the current app session. Use Tab and Shift+Tab to move
focus and Enter or Space to activate the focused link. Moving the pointer over
a link focuses it, and left-clicking activates it. Empty and `#fragment` targets
remain visible but are intentionally not focusable.

`Input` and `TextArea` are controlled components: pass the displayed `value`
from caller-owned state and update that state from `on_input` when editing
proposes a new value. Both support optional `placeholder` text. Wrap editable
controls in `Form` to centralize submit and cancel behavior. Focused inputs
submit with Enter; focused text areas insert a newline with Enter in insert
mode and submit with Ctrl+Enter. Pressing Esc from editable normal mode cancels
the nearest form when it has an `on_cancel` handler.

Editable controls start in a compact Vim-style normal mode. Use `i`, `a`, `I`,
or `A` to enter insert mode; Esc or `jk` returns to normal mode. Normal and
visual mode support common movement (`h`, `j`, `k`, `l`, `0`, `$`, `w`, `b`,
`e`, `gg`, `G`), selection (`v`, `V`), delete/yank/paste (`x`, `d`, `dd`,
`y`, `yy`, `p`), line opening (`o`, `O` for text areas), undo (`u`), and redo
(Ctrl+R).

`Image` renders a path-backed terminal image when the active terminal exposes a
supported graphics protocol. Otherwise it renders deterministic fallback text,
preferring the view's `alt` text when provided. `ProgressBar` renders a gauge
with a clamped `0.0..=1.0` value and optional `label`.

## Semantic Documents

Semantic document views include `H1` through `H6`, `Paragraph`, ordered and
unordered lists, responsive tables, and `CodeBlock`. Lists contain block-based
`ListItem` children, while tables use `TableHead` or `TableBody`, `TableRow`,
and inline-text `TableCell` values. Builders compose the same tree as the
PascalCase `view!` tags:

```rust
use leptatui::prelude::*;

let document = div((
    h1("Leptatui guide"),
    paragraph("Semantic content wraps with the terminal viewport."),
    ordered_list([
        list_item((
            paragraph("Compose block-oriented list items."),
            unordered_list([list_item([paragraph("Nest list types freely.")])]),
        )),
        list_item([paragraph("Choose the first decimal marker.")]),
    ])
    .start(3),
    table([
        table_head([table_row([
            table_cell("Component"),
            table_cell("Status").alignment(CellAlignment::Center),
        ])]),
        table_body([table_row([
            table_cell("CodeBlock"),
            table_cell("Ready").alignment(CellAlignment::Right),
        ])]),
    ]),
    code_block("fn main() { println!(\"hello\"); }")
        .language("rust")
        .line_numbers(true),
));
```

The same document shape can be written and styled with nested tags. Semantic
view names are also stylesheet type selectors:

```rust
use leptatui::prelude::*;

#[component]
fn Guide() -> impl IntoView {
    stylesheet! {
        H1 => { fg: Color::LightCyan }
        OrderedList => { fg: Color::LightGreen }
        TableHead => { fg: Color::LightCyan }
        CodeBlock => { fg: Color::LightBlue }
    }

    view! {
        <Div>
            <H1>"Leptatui guide"</H1>
            <OrderedList start=3>
                <ListItem>
                    <Paragraph>"Compose block-oriented list items."</Paragraph>
                    <UnorderedList>
                        <ListItem><Paragraph>"Nest list types freely."</Paragraph></ListItem>
                    </UnorderedList>
                </ListItem>
            </OrderedList>
            <Table>
                <TableHead>
                    <TableRow>
                        <TableCell>"Component"</TableCell>
                        <TableCell alignment={CellAlignment::Center}>"Status"</TableCell>
                    </TableRow>
                </TableHead>
                <TableBody>
                    <TableRow>
                        <TableCell>"CodeBlock"</TableCell>
                        <TableCell alignment={CellAlignment::Right}>"Ready"</TableCell>
                    </TableRow>
                </TableBody>
            </Table>
            <CodeBlock
                language="rust"
                line_numbers=true
            >"fn main() {}"</CodeBlock>
        </Div>
    }
}
```

Headings, paragraphs, list items, table cells, and code lines wrap to the
available width and contribute their wrapped height to parent layouts. Code
blocks do not scroll horizontally. A recognized `language` selects a bundled
syntax grammar; an unknown language keeps the source readable as plain text.
Syntax colors use the terminal's ANSI palette, code-block backgrounds use the
terminal's `DarkGray` palette entry, and line numbers are disabled unless
requested. Table cells accept inline Ratatui text rather than nested block views
in the v1 API.

## Markdown Readers

Use `markdown` for default in-memory conversion or `markdown_with_options` to
configure every parsed code block:

```rust
use leptatui::prelude::*;

let source = "# Guide\n\n```rust\nfn main() {}\n```";
let default_document = markdown(source);
let highlighted_document = markdown_with_options(
    source,
    MarkdownOptions::default().line_numbers(true),
);
```

For synchronous UTF-8 path loading, use `markdown_file` or
`markdown_file_with_options`. The path-backed `Markdown` tag instead loads
asynchronously through `use_file_system()` at the containing volume root:

```rust
use leptatui::prelude::*;

let default_file = markdown_file("README.md");
let configured_file = markdown_file_with_options(
    "README.md",
    MarkdownOptions::default().line_numbers(true),
);
let tagged_file = view! {
    <Markdown src="README.md" editable=true line_numbers=true />
};
```

`editable` defaults to `false`. When enabled, the element consumes an
unmodified `e` to open its original `src` through `use_editor()` and an
unmodified `r` to refetch that source. A successful editor session also
refetches the source. Each refetch replaces the Markdown view, resetting its
local navigation history, focus, and scroll state. Editor and reload failures
use the same standard error screen as the initial asynchronous read. If a view
contains multiple editable Markdown elements, normal view-tree traversal gives
the first eligible element the shortcut.

Markdown readers use the same semantic defaults as their underlying elements:
cyan-to-gray heading levels, white paragraphs, cyan ordered lists and table
headers, green unordered lists, terminal-native syntax on dark-gray code blocks,
blue Markdown links, and white-on-dark-gray focused links and anchors. Visited
Markdown links are magenta and underlined; active route anchors are light cyan
and bold. The same palette applies to
standalone tags such as `<H1>`,
`<Paragraph>`, `<CodeBlock>`, and `<A>`. Authored stylesheet rules and inline
styles override these defaults through the normal cascade.

The compatibility promise is CommonMark plus tables. Optional GFM task lists,
strikethrough, footnotes, and other extensions are deferred. Markdown links
retain their inline labels and are navigable with the same focus and activation
controls as standalone `Link` views, including pointer focus and left-click
activation. The mouse wheel scrolls overflowing document content under the
pointer. In a file-backed reader, relative and absolute `.md` or `.markdown`
targets open inside the same app boundary. Non-empty fragments scroll to
GitHub-style heading anchors; duplicate headings receive `-1`, `-2`, and later
suffixes. Use Shift+H and Shift+L to move backward and forward through cached
pages, restoring each page's focus and scroll state.
URLs and other local files use the system handler. Standalone links and
in-memory Markdown keep that external behavior. Empty destinations and a bare
`#` remain inactive.
Images become deterministic descriptive text; local and remote image targets
are never fetched. Raw HTML and unsupported blocks remain readable fallbacks.
Synchronous file readers render unreadable paths and invalid UTF-8 as a
path-aware in-app page. The asynchronous `Markdown` tag shows its loading state
and then raises a `ViewError`, producing the standard interactive error screen.

```rust
use leptatui::prelude::*;

#[component]
fn StandardControls() -> impl IntoView {
    let name = RwSignal::new(String::from("Ada Lovelace"));
    let notes = RwSignal::new(String::from("Sketch the first program."));
    let progress = RwSignal::new(0.4);

    view! {
        <Div>
            {move || {
                let name_value = name.get();
                let notes_value = notes.get();

                view! {
                    <Form on_submit=|| AppControl::Continue>
                        <Input
                            value=name_value
                            placeholder="Name"
                            on_input=move |next| {
                                name.set(next);
                                AppControl::Continue
                            }
                        />
                        <TextArea
                            value=notes_value
                            placeholder="Notes"
                            on_input=move |next| {
                                notes.set(next);
                                AppControl::Continue
                            }
                        />
                    </Form>
                }
            }}
            <Image
                src="crates/leptatui/examples/assets/showcase.jpg"
                alt="Image fallback text"
            />
            {move || progress_bar(progress.get()).label("Progress")}
        </Div>
    }
}
```

Buttons, inputs, and text areas default to white text with rounded borders and
one cell of horizontal padding. Focused controls use bold white text on the
terminal surface background with thick borders; insert-mode inputs and text
areas use yellow foregrounds, while both visual modes use magenta foregrounds.
Tables default to white and progress bars to light green on dark gray. Links are
blue and underlined until visited, then magenta and underlined. These remain
low-precedence defaults.

[Documentation index](../README.md) · [Feature index](features.md)
