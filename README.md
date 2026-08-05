# Leptatui

Leptatui is an experimental Rust terminal UI runtime that combines Leptos
reactive primitives with Ratatui rendering and Crossterm event handling. It
provides a small view tree, component contract, styling helpers, and procedural
macros for building interactive terminal applications.

## Workspace Layout

- `crates/leptatui`: Public runtime crate with app, component, view, context,
  style, and prelude APIs.
- `crates/leptatui-macros`: Internal proc-macro crate that implements
  `#[component]`, `view!`, `view_error!`, and `stylesheet!`.
- `crates/leptatui/examples`: Runnable examples for the public crate.

Within each crate, domain facades keep public imports stable while substantive
subdirectories group related implementations. Generic pass-through layers such
as `model`, `utils`, or `expansion` are avoided; a directory either owns direct
source files or is flattened into its parent.

## Quality Gates

CI validates the full workspace with the same commands expected for local
pre-review checks:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
cargo check --workspace --examples
```

## Usage Shape

Application code normally imports `leptatui::prelude::*`, defines a root
component, builds a view tree with either builders or `view!`, and runs it with
`App::new(view).run().await`.

`view!` and `#[component]` are Leptatui-owned macros for terminal UIs. They use
familiar Leptos-style component syntax, but they create values implementing
Leptatui's `View` trait instead of Leptos DOM nodes.

Generated `#[component]` bodies run once when `new()` creates the component,
under a stored Leptos owner. Create signals directly in the component body, and
read them from dynamic views or event handlers when values need to update.
Buttons support Tab/Shift+Tab focus movement, Enter/Space activation, pointer
focus, and left-click activation by default. The mouse wheel scrolls the
overflowing layout under the pointer. Register handlers with `use_key_event()`
inside a component body for custom key maps and overrides.

```rust
use leptatui::prelude::*;

#[component]
fn Greeting() -> impl IntoView {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') {
            return KeyControl::Exit;
        }

        KeyControl::Pass
    });

    view! { <Text>"Hello from Leptatui"</Text> }
}

#[tokio::main]
async fn main() -> leptatui::app::Result<()> {
    let view = view! { <Greeting /> };
    App::new(view).run().await
}
```

Components that perform fallible setup can return
`ViewResult<impl IntoView>`. The component macro permits `?` and automatically
wraps the final bare view expression in `Ok`. An error replaces the active
application UI with a standalone full-screen diagnostic and router-aware Back
and Quit controls:

```rust
use leptatui::prelude::*;

#[component]
fn Document() -> ViewResult<impl IntoView> {
    let source = std::fs::read_to_string("guide.md")?;
    view! { <Text>{source}</Text> }
}
```

Use `view_error!` for an intentional custom diagnostic. Passing only a message
replaces the underlying failure; the `error => message` form preserves the
source beneath anyhow context:

```rust
if let Err(error) = std::fs::read_to_string("settings.toml") {
    view_error!(error => "Could not load application settings");
}
```

The error screen supports Tab/Shift+Tab and Enter/Space, `Esc` or `b` to go
back, and `q` to quit. While the managed app is active, panic handling restores
the normal terminal before forwarding the panic to Rust's existing hook.

Signals created in a component body live for that component instance. Provide
shared signals or services through typed context when descendants need them.
Components that must temporarily release the terminal—for example, to launch an
interactive editor—can call `use_app_handle()` and queue work with
`AppHandle::suspend_terminal()`. The runtime restores normal terminal modes,
runs the work on the app thread, re-enters the TUI, and redraws the same mounted
component tree.

Component function parameters are props. Use PascalCase component tags in
`view!`, pass props as attributes, and use nested content with a `children:
Children` prop. `#[prop(optional)]`, `#[prop(default = ...)]`, and
`#[prop(into)]` follow the same shape as Leptos component props.

```rust
#[component]
fn Label(#[prop(into)] text: String) -> impl IntoView {
    view! { <Text>{text}</Text> }
}

#[component]
fn Panel(#[prop(into)] title: String, children: Children) -> impl IntoView {
    view! {
        <Block>
            <Div>
                <Text>{title}</Text>
                {children()}
            </Div>
        </Block>
    }
}

view! {
    <Panel title="Theme variables">
        <Label text="Active theme" />
    </Panel>
}
```

## Standard Components

Leptatui's standard view set includes layout containers (`Block`, `Div`),
plain and semantic text, nested lists, responsive tables,
syntax-highlighted code blocks, CommonMark documents, buttons, controlled form
controls, images, and progress bars.
All are available through `leptatui::prelude::*` as builders and through
PascalCase `view!` tags.

`View` is an open, object-safe trait. Built-in builders retain concrete return
types such as `TextView`, `DivView`, `InputView`, and `TextAreaView`, so their
type-specific configuration remains available without pattern matching.
Containers accept homogeneous collections or heterogeneous tuples through
`IntoViews`. Braced `Vec<V>` expressions inside `Div`, `Form`, `ListItem`, or
component children splice their views directly into the surrounding child list.
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

### Semantic Documents

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

### Markdown Readers

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

For explicit UTF-8 path loading, use `markdown_file`,
`markdown_file_with_options`, or the path-backed `Markdown` tag. Loading
finishes before the view is returned:

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
Unreadable paths and invalid UTF-8 also render a path-aware in-app page instead
of returning an error, so back navigation remains available.

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
                let name_value = name.get_untracked();
                let notes_value = notes.get_untracked();

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
            {move || progress_bar(progress.get_untracked()).label("Progress")}
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

Styles live with components. Invoke `stylesheet!` directly inside a
`#[component]` body, or call a synchronous helper that invokes it, to register
those rules for that component subtree, including descendant components. The
helper must run during component setup, before the component returns its view.
The same macro still returns a `Stylesheet` value for direct construction and
tests. It supports flat terminal selectors and nested rules that lower into
explicit descendant selectors. Use `&:focus`, `&:active`,
`&:insert`, `&:visual`, or `&:visited` inside a nested rule to combine a
pseudo-class with the current terminal selector. `:insert` matches `Input` and
`TextArea` views whose retained Vim mode is `Insert`; `:visual` matches either
character-wise Visual or Visual Line mode; `:visited` matches links whose
destination was opened during the current app session. Matching rules use
CSS-style specificity and source order; inline styles override normal
stylesheet declarations, and stylesheet declarations marked `!important`
override normal inline styles.

Larger components can keep their Rust stylesheet in a sibling `style.rs`.
Styled child components use a directory containing `component.rs`, `style.rs`,
and `mod.rs`; routed pages use the equivalent `page.rs`, `style.rs`, and
`mod.rs`. Components without local rules can omit `style.rs` and remain in a
flat module.

```text
example_card/
├── component.rs
├── mod.rs
└── style.rs
```

```rust
// example_card/mod.rs
mod component;
mod style;

pub use component::ExampleCard;

// example_card/component.rs
use leptatui::prelude::*;

use super::style::use_example_card_styles;

/// Renders an example card with co-located styles.
#[component]
pub fn ExampleCard() -> impl IntoView {
    use_example_card_styles();

    view! {
        <Div class="example-card">
            <Text class="example-card__content">"Example card"</Text>
        </Div>
    }
}

// example_card/style.rs
use leptatui::prelude::*;

/// Registers the example card stylesheet with the active component.
pub(super) fn use_example_card_styles() {
    stylesheet! {
        .example-card => {
            &__content => { fg: Color::LightCyan }
        }
    };
}
```

The trailing semicolon in the helper keeps its return type as `()`;
`stylesheet!` remains value-producing for direct stylesheet construction.

```rust
#[component]
fn Panel() -> impl IntoView {
    stylesheet! {
        .panel => {
            bg: Color::Black

            Text => { fg: Color::White }

            Button => {
                &:focus => { bg: Color::Yellow !important }
            }
        }
    }

    view! { <Block class="panel"><Button>"Save"</Button></Block> }
}
```

Nested class rules also support BEM-style `&__element` and `&--modifier`
suffixes. These concatenate with the nearest parent class rather than creating
a descendant selector, and may be chained or combined with the supported
pseudo-classes. Parent suffixes require a class selector; they cannot be used at
the top level or beneath a type or id selector.

```rust
#[component]
fn ExamplePage() -> impl IntoView {
    stylesheet! {
        .example-page => {
            &__button => {
                &:focus => { bg: Color::Black }
                &--submit => { fg: Color::Green }
                &--cancel => { fg: Color::Red }
            }
        }
    }

    view! {
        <Div class="example-page">
            <Button class="example-page__button example-page__button--submit">
                "Submit"
            </Button>
            <Button class="example-page__button example-page__button--cancel">
                "Cancel"
            </Button>
        </Div>
    }
}
```

Stylesheets support media queries nested within selector blocks for responsive
terminal UIs. Direct declarations apply to the containing selector, while
nested rules retain its selector path. Width and height values are terminal-cell
counts from the root viewport. Top-level media blocks remain supported for
backward compatibility.

```rust
#[component]
fn ResponsivePanel() -> impl IntoView {
    stylesheet! {
        .panel => {
            padding: TuiSpacing::uniform(1)

            @media (max-width: 80) {
                padding: TuiSpacing::ZERO
                Text => { fg: Color::Yellow }
            }
        }
        .actions => {
            display: Display::Flex

            @media (max-width: 80) {
                flex_direction: FlexDirection::Column
            }
        }
        Button:focus => {
            @media (min-width: 81) and (min-height: 24) {
                bg: Color::Yellow
            }
        }
    }

    view! {
        <Block class="panel">
            <Div>
                <Text>"Devtools"</Text>
                <Div class="actions">
                    <Button>"Inspect"</Button>
                    <Button>"Quit"</Button>
                </Div>
            </Div>
        </Block>
    }
}
```

### Layout and Typed Styles

Leptatui exposes layout as Rust values rather than parsed CSS strings or
layout-engine types. Apply a `TuiStyle` directly with a view's `style`
attribute or declare the same properties in `stylesheet!`. Inline and
stylesheet declarations both resolve into one computed style before every
root layout pass.

```rust
use leptatui::prelude::*;

let inline = TuiStyle::new()
    .display(Display::Flex)
    .size(LayoutSize::new(
        Dimension::from(Length::percent(100.0)),
        Dimension::from(Length::vh(50.0)),
    ))
    .gap(Axes::all(Length::cells(1.0)))
    .overflow(Axes::new(Overflow::Hidden, Overflow::Auto));

let panel = view! {
    <Div style={inline}>
        <Text>"Typed layout values"</Text>
    </Div>
};
# let _ = panel;
```

The public layout properties and their value types are:

| Properties | Public value types |
| --- | --- |
| `display`, `box_sizing`, `overflow` | `Display`, `BoxSizing`, `Axes<Overflow>` |
| `size`, `min_size`, `max_size`, `aspect_ratio` | `LayoutSize<Dimension>`, `f32` |
| `margin`, `padding`, `gap` | `Edges<LengthAuto>`, `TuiSpacing`, `Axes<Length>` |
| `flex_direction`, `flex_wrap` | `FlexDirection`, `FlexWrap` |
| `flex_basis`, `flex_grow`, `flex_shrink` | `Dimension`, `f32` |
| `align_items`, `align_self`, `align_content` | `AlignItems`, `AlignSelf`, `AlignContent` |
| `justify_content`, `justify_items`, `justify_self` | `JustifyContent`, `JustifyItems`, `JustifySelf` |
| `grid_template_rows`, `grid_template_columns` | `Vec<GridTemplateTrack>` |
| `grid_auto_rows`, `grid_auto_columns`, `grid_auto_flow` | `Vec<GridTrackSize>`, `GridAutoFlow` |
| `grid_row`, `grid_column` | `GridLine` |
| `position`, `inset`, `z_index` | `Position`, `Edges<LengthAuto>`, `ZIndex` |

`Length` supports terminal cells, containing-block percentages, and `vw`,
`vh`, `vmin`, and `vmax` terminal viewport units. `Dimension` adds `Auto`,
`MinContent`, `MaxContent`, and `FitContent`. General box dimensions currently
treat min-content and max-content like auto, and fit-content like its contained
length; grid track sizing supports its intrinsic variants directly. Non-finite
values are sanitized before reaching layout. Negative `Length` and factor
values are clamped to zero, while non-positive aspect ratios fall back to
automatic sizing.

### Layout Conformance Matrix

The layout contract is protected by public-API integration tests that retain
terminal geometry and, where painting matters, compare complete rendered rows.
This matrix is the index for the supported property groups and the edge cases
that must remain covered:

| Contract | Properties and behavior | Edge-case coverage | Executable coverage |
| --- | --- | --- | --- |
| Flow and visibility | `display`, block flow, source ordering, `Display::None` | Automatic sizes, nested blocks, transparent boundaries, hidden subtrees | `cargo test -p leptatui --test view suite::layout::block_flow` |
| Box geometry | `box_sizing`, `size`, `min_size`, `max_size`, `aspect_ratio`, `margin`, `padding` | Content/border boxes, percentages, viewport units, invalid ratios, zero-sized boxes, cumulative rounding | `cargo test -p leptatui --test view suite::layout::box_model`<br>`cargo test -p leptatui --test view suite::layout::sizing` |
| Intrinsic measurement | Automatic dimensions and measured leaf/container contributions | Wrapped text, trailing newlines, component boundaries, nested/replaced views | `cargo test -p leptatui --test view suite::layout::measurement` |
| Overflow | `overflow` on both axes, retained viewport and clip geometry | Visible/hidden/clip/scroll/auto, conditional gutters, nested clips, wheel/focus scrolling, reconciliation | `cargo test -p leptatui --test view suite::layout::overflow` |
| Flexbox | `flex_direction`, `flex_wrap`, `flex_basis`, `flex_grow`, `flex_shrink`, `gap`, flex alignment | Reversed axes, wrapped lines, intrinsic bases, constraints, zero-sized items, odd remainders, terminal resize | `cargo test -p leptatui --test flexbox_conformance`<br>`cargo test -p leptatui --test view suite::layout::flex` |
| Grid sizing | Templates, automatic tracks, `minmax()`, repeats, fractions, `gap`, grid alignment | Intrinsic and nested grids, percentage/auto/fraction mixes, empty repeated tracks, cumulative rounding, terminal resize | `cargo test -p leptatui --test grid_sizing_conformance` |
| Grid placement | `grid_row`, `grid_column`, `grid_auto_flow` | Signed lines, forward/backward spans, implicit tracks, collisions, sparse/dense row and column flow | `cargo test -p leptatui --test grid_placement_conformance` |
| Positioning and stacking | `position`, `inset`, `z_index` | Static/relative/absolute/fixed/sticky, containing blocks, percentage resize, nested scrollports, clipping, atomic stacking contexts | `cargo test -p leptatui --test positioning_conformance` |
| Responsive recomputation | Viewport units, media queries, retained geometry rebuilds | Narrow/wide breakpoints, repeated resize, flex/grid reflow | `cargo test -p leptatui --test view suite::layout::responsive`<br>`cargo test -p leptatui --test flexbox_conformance responsive`<br>`cargo test -p leptatui --test grid_sizing_conformance responsive` |

The performance suite exercises the same public render path for cold layout
construction, intrinsic measurement, resize recomputation, deep trees, and
large flex/grid collections. Record a machine-local Criterion baseline before
making layout changes:

```sh
cargo bench -p leptatui --bench layout -- --save-baseline phase-18
```

Compare a later run on the same machine and toolchain:

```sh
cargo bench -p leptatui --bench layout -- --baseline phase-18
```

Criterion stores named baselines below the ignored `target/criterion`
directory. Timings are intentionally not committed because terminal backend,
host, power, and toolchain differences make cross-machine measurements
incomparable.

### Div and Box Geometry

`Div` is the generic multi-child layout container. It defaults to
`Display::Block`, so direct children stack vertically in source order.
`Block` is a bordered single-child container that participates in the same
computed layout. Use `Div` for structure and `Block` when the container itself
needs Ratatui border chrome. `Display::Flex` and `Display::Grid` change how a
container places direct children; `Display::None` removes the entire subtree
from measurement, painting, focus, and input.

Every visible styleable view retains a `LayoutGeometry` after the root layout
pass. `border_box` includes content, padding, and borders; `padding_box`
excludes borders; `content_box` excludes borders and padding; `viewport`
subtracts scrollbar gutters; and `clip` records the accumulated ancestor
clip. `BoxSizing::ContentBox` applies authored sizes to the content box, while
`BoxSizing::BorderBox` includes padding and borders in those sizes. Margins
remain outside the retained border box.

Layout is calculated with floating-point values and then retained as terminal
cell rectangles. Siblings and tracks are rounded cumulatively, preserving
contiguous edges and assigning the final remainder to the sequence instead of
rounding every item independently.

Run the block flow, box sizing, and rounding example:

```sh
cargo run --example block_layout
```

### Overflow Contract

Overflow is configured independently for the horizontal and vertical axes.
Without an authored value, layout uses visible horizontal overflow and
automatic vertical overflow.

- `Overflow::Visible` paints outside the content viewport without clipping or
  scrolling.
- `Overflow::Hidden` clips excess content and retains programmatic,
  keyboard, focus, and wheel scrolling.
- `Overflow::Clip` clips without creating a scroll container.
- `Overflow::Scroll` always enables scrolling and reserves a one-cell terminal
  scrollbar gutter.
- `Overflow::Auto` clips, scrolls, and reserves a gutter only when direct child
  geometry exceeds the available axis.

Scroll ranges are derived from direct layout children; visible overflow inside
a child does not enlarge its parent's range. Nested clips compose during
painting and hit testing. Pointer wheel events target the frontmost
overflowing box under the pointer and bubble to an ancestor at a boundary.
Vertical scrolling also supports Up/Down, `j`/`k`, Page Up/Page Down,
Ctrl-U/Ctrl-D, `gg`, and `G`. Focus traversal scrolls controls into view.

Run the nested two-axis overflow example:

```sh
cargo run --example nested_overflow
```

### Flexbox Contract

Set `display: Display::Flex` on a container to lay out its direct children on
one main axis. Flex containers default to `FlexDirection::Row` and
`FlexWrap::NoWrap`. Items default to an automatic intrinsic basis, no growth,
and a shrink factor of `1.0`; gaps default to zero. Container alignment uses
the layout engine defaults: items and lines stretch on the cross axis, while
main-axis content starts at the selected edge.

The public flex contract includes:

- `flex_direction`: row, row-reverse, column, and column-reverse.
- `flex_wrap`: no-wrap, wrap, and wrap-reverse.
- `gap`: independent horizontal and vertical item or line gaps.
- `flex_basis`, `flex_grow`, and `flex_shrink`: preferred item size and
  positive or negative free-space distribution.
- `justify_content`: main-axis packing and space distribution.
- `align_items` and `align_self`: container and per-item cross-axis alignment.
- `align_content`: wrapped-line distribution on the cross axis.

Text, controls, blocks, and nested layout containers participate through their
intrinsic measurements. Media queries can change flex properties when the
terminal viewport changes, causing layout and measurement to run again.
Engine-owned fractional geometry is rounded cumulatively when retained as
terminal rectangles so adjacent children remain contiguous and the final item
ends at the expected parent edge.

Run the responsive navigation, content, and sidebar example:

```sh
cargo run --example responsive_flex
```

### Positioning Contract

The `position` property selects how a box participates in layout and how its
`inset` edges are resolved. `Position::Static` is the default and ignores
insets. `Position::Relative` keeps its normal-flow space while offsetting its
painted box. `Position::Absolute` leaves normal flow and uses the nearest
non-static ancestor, or the layout root, as its containing block.
`Position::Fixed` also leaves normal flow but is positioned and clipped
against the terminal viewport. `Position::Sticky` keeps its flow space and
clamps its painted box to the authored inset inside the nearest scrollport.

Automatic insets preserve the box's static source position on that axis.
Percentage insets resolve against the relevant containing block and are
recomputed after terminal resizes. Positioned descendants retain ancestor
clipping and scrolling unless fixed positioning explicitly moves them into
the viewport layer.

Positioned boxes with `ZIndex::Auto` share their containing stacking context
and paint in source order at the automatic level. `ZIndex::Integer` assigns a
signed stacking level and establishes an atomic local stacking context,
including for an explicit value of zero. Negative positioned layers paint
after their context's background but before normal-flow content, while
positive layers paint afterward. Pointer targeting follows the resulting
paint order.

Run the static, relative, absolute, fixed, sticky, and stacking example:

```sh
cargo run --example positioning_showcase
```

### Grid Contract

Set `display: Display::Grid` on a container to lay out its direct children in
explicit or automatically created rows and columns. Grid templates use
Leptatui-owned track values rather than exposing layout-engine types. Empty
templates create tracks as automatic placement requires, automatic flow
defaults to row-major sparse packing, and row and column gaps default to zero.

The public grid contract includes:

- `grid_template_columns` and `grid_template_rows`: fixed-cell, percentage,
  fractional, automatic, min-content, max-content, `minmax`, and repeated
  explicit tracks.
- `grid_auto_columns` and `grid_auto_rows`: cyclic sizing patterns for implicit
  tracks created outside the explicit template.
- `grid_auto_flow`: sparse or dense row-major and column-major placement.
- `grid_column` and `grid_row`: signed explicit line pairs, automatic edges,
  and forward or backward spans.
- `gap`: independent horizontal and vertical spacing between tracks.
- `justify_content` and `align_content`: positioning and distribution of the
  grid track area inside the container.
- `justify_items`, `align_items`, `justify_self`, and `align_self`: container
  defaults and per-item alignment overrides.

Text and nested containers contribute intrinsic measurements to automatic and
intrinsic tracks. Fixed tracks and gaps reserve space before fractions consume
the remainder, while item min/max sizes constrain the rectangle inside its
assigned grid area. Media-query changes and terminal resizes rebuild the
template and retained geometry. Fractional engine coordinates are rounded
cumulatively into terminal rectangles so adjacent tracks remain contiguous and
the final track reaches the parent edge.

Run the responsive dashboard example:

```sh
cargo run --example responsive_grid
```

### Supported CSS Differences

Leptatui implements a deliberately typed, terminal-specific subset of
web-style layout. Terminal cells are the absolute unit, all edges are physical
rather than writing-mode-relative, and no DOM, CSS parser, cascade origins,
animations, floats, multicolumn layout, or browser compatibility layer are
provided. Styles are authored through Rust builders and `stylesheet!`; the
underlying layout engine remains private.

Padding and borders are integral terminal cells, scrollbars occupy a cell
gutter, text measurement uses terminal display width, and painting is clipped
to the terminal buffer. Fixed boxes use the terminal viewport rather than a
browser visual viewport. Sticky boxes clamp inside the nearest Leptatui
scrollport. Z-index affects positioned boxes only, and pointer targeting
follows the final terminal paint order. The current API is the supported
contract; superseded row/column splitters and height-only layout paths are not
retained as aliases.

Reusable declaration groups can be declared with `@mixin` and expanded in rule
bodies with `@include`.

```rust
#[component]
fn MixedPanel() -> impl IntoView {
    stylesheet! {
        @mixin panel_chrome {
            bg: Color::Black,
            padding: TuiSpacing::uniform(1)
        }

        @mixin focused_control {
            fg: Color::Black,
            bg: Color::Yellow
        }

        .panel => {
            @include panel_chrome

            Text => { fg: Color::White }

            Button => {
                &:focus => { @include focused_control }
            }
        }
    }

    view! { <Block class="panel"><Button>"Save"</Button></Block> }
}
```

Variables and mixins can also live in helper functions and be imported with
`@use`. Imports use the helper function name as their namespace by default, or
can be renamed with `as`. A `stylesheet!` invocation with variables or mixins
but no rules returns a `StyleModule`.

```rust
fn color_variables() -> StyleModule {
    stylesheet! {
        $fg: Color::Black;
        $bg: Color::White;
    }
}

fn button_mixins() -> StyleModule {
    stylesheet! {
        @use color_variables;

        @mixin primary {
            fg: color_variables.$fg,
            bg: color_variables.$bg,
            borders: Borders::ALL,
            border_type: BorderType::Rounded,
            padding: TuiSpacing::uniform(1)
        }

        @mixin inverted {
            @include primary,
            fg: color_variables.$bg,
            bg: color_variables.$fg
        }
    }
}

#[component]
fn Actions() -> impl IntoView {
    stylesheet! {
        @use button_mixins as button;

        .submit => { @include button.primary }
        .quit => { @include button.inverted }
    }

    view! {
        <Div style={TuiStyle::new().display(Display::Flex)}>
            <Button class="submit">"Submit"</Button>
            <Button class="quit">"Quit"</Button>
        </Div>
    }
}
```

Stylesheets can also reference runtime theme variables. Define each theme as a
`ThemeVariables` value, provide either that value or a
`ReadSignal<ThemeVariables>` through context, then use `theme_color("name")` in
stylesheet declarations. Theme values are resolved during rendering, so a root
component can switch the active theme signal and descendants repaint with the
new values on the next draw without hardcoding per-theme colors in component
view code.

Keep literal colors in the theme model and keep component stylesheets written
against variable names:

```rust
#[component]
fn ThemedPanel() -> impl IntoView {
    provide_context(
        ThemeVariables::new()
            .color("text", Color::Black)
            .color("surface", Color::White),
    );

    stylesheet! {
        $text: theme_color("text");
        $surface: theme_color("surface");

        .panel => { fg: $text, bg: $surface }
    }

    view! { <Block class="panel"><Text>"Theme-aware"</Text></Block> }
}
```

For runtime switching, provide the active mode and active variables as context
signals near the root. Descendant components can consume the mode for labels or
controls, while stylesheet resolution consumes `ReadSignal<ThemeVariables>` for
colors:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ThemeMode {
    Light,
    Dark,
}

impl ThemeMode {
    fn toggle(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }

    fn variables(self) -> ThemeVariables {
        match self {
            Self::Light => ThemeVariables::new()
                .color("text", Color::Black)
                .color("surface", Color::White),
            Self::Dark => ThemeVariables::new()
                .color("text", Color::White)
                .color("surface", Color::Black),
        }
    }
}

#[component]
fn ThemeLabel() -> impl IntoView {
    let mode = expect_context::<ReadSignal<ThemeMode>>();

    dynamic(move || {
        view! { <Text>{format!("Theme: {:?}", mode.get_untracked())}</Text> }
    })
}

#[component]
fn ThemeRoot() -> impl IntoView {
    let mode = RwSignal::new(ThemeMode::Light);
    let theme = RwSignal::new(ThemeMode::Light.variables());

    provide_context(mode.read_only());
    provide_context(theme.read_only());

    stylesheet! {
        $text: theme_color("text");
        $surface: theme_color("surface");

        .panel => { fg: $text, bg: $surface }
    }

    view! {
        <Block class="panel">
            <Div>
                <ThemeLabel />
                <Button on_press=move || {
                    mode.update(|mode| {
                        *mode = mode.toggle();
                        theme.set(mode.variables());
                    });
                    AppControl::Continue
                }>
                    "Toggle theme"
                </Button>
            </Div>
        </Block>
    }
}
```

See `cargo run --example theme_switcher` for the complete light/dark theme
switcher.

Multi-page apps use URL-like locations and declarative route components. Wrap
the application shell in `Router`, place route declarations inside `Routes`,
and navigate with internal `A` anchors or `use_navigate()`. Content outside
`Routes` remains mounted, and `ParentRoute` plus `Outlet` provides retained
nested layouts. An `A` whose destination matches the current route matches the
`:active` stylesheet pseudo-class. Route matching no longer injects an
`active` class, so authored route-state rules should use `A:active` or nested
`&:active` selectors.

Use props for required parent-to-child inputs and local component
configuration. Use context for app-wide state that many routes or deeply nested
descendants need to read or update, such as theme variables or persisted
settings. Keep shared state owned by the root when navigation
should not reset it; add explicit reset behavior when a route change should
clear shared state. Descendant pages can read required context with
`expect_context()` or optional context with `use_context()`.

```rust
use leptatui::prelude::*;

#[component]
fn Nav() -> impl IntoView {
    view! {
        <Div style={TuiStyle::new().display(Display::Flex)}>
            <A href="/" exact=true>"Home"</A>
            <A href="/counter">"Counter"</A>
            <A href="/settings">"Settings"</A>
        </Div>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    let counter = expect_context::<RwSignal<i32>>();

    view! {
        <Div>
            <Text>"Home"</Text>
            {move || {
                view! {
                    <Text>{format!("Count: {}", counter.get_untracked())}</Text>
                }
            }}
        </Div>
    }
}

#[component]
fn CounterPage() -> impl IntoView {
    let counter = expect_context::<RwSignal<i32>>();

    view! {
        <Div>
            <Text>"Counter"</Text>
            <Button on_press=move || {
                counter.update(|count| *count += 1);
                AppControl::Continue
            }>
                "Increment"
            </Button>
        </Div>
    }
}

#[component]
fn SettingsPage() -> impl IntoView {
    let location = use_location();

    view! {
        <Text>{move || format!("Current path: {}", location.pathname().get())}</Text>
    }
}

#[component]
fn NotFoundPage() -> impl IntoView {
    view! { <Text>"Page not found"</Text> }
}

#[component]
fn Root() -> impl IntoView {
    let counter = RwSignal::new(0);

    provide_context(counter);

    view! {
        <Router initial_path="/">
            <Div>
                <Nav />
                <Routes fallback=NotFoundPage>
                    <Route path="/" view=HomePage />
                    <Route path="/counter" view=CounterPage />
                    <Route path="/settings" view=SettingsPage />
                </Routes>
            </Div>
        </Router>
    }
}

#[tokio::main]
async fn main() -> leptatui::app::Result<()> {
    let view = view! { <Root /> };
    App::new(view).run().await
}
```

Route patterns accept static segments, `:parameter` segments, and a terminal
`*wildcard`. Read decoded matches with `use_params_map()`, query values with
`use_query_map()`, and the current path with `use_location()`. Programmatic
navigation uses `use_navigate()`, while `use_history()` exposes process-local
back and forward controls.

See `crates/leptatui/examples/multi_page_demo.rs` or run
`cargo run --example multi_page_demo` for a complete routing and context
example with shared counter and theme state.

## Resources And Actions

Use `Resource::new` for asynchronous reads keyed by reactive source state and
`Action::new` for POST-like mutations triggered by buttons or key handlers.
Both types publish signal-backed loading, input, and value state and request
redraws when work changes. Fetchers and handlers can return any output type;
application failures remain ordinary `Result<T, E>` values handled by
components or contexts.

Resources retain their previous value while reloading and ignore stale
completions from older source keys. Actions retain their previous output,
expose the current input only while pending, and ignore stale completions. A
common pattern is to keep a refresh signal in root context, key the resource
from that signal, and increment it after a successful action.

```rust
use leptatui::prelude::*;

#[derive(Clone)]
struct Todos {
    items: Resource<Result<Vec<String>, String>>,
    create: Action<String, Result<String, String>>,
    refresh: WriteSignal<u64>,
}

async fn load_todos() -> std::result::Result<Vec<String>, String> {
    Ok(vec![String::from("Write terminal docs")])
}

async fn create_todo(title: String) -> std::result::Result<String, String> {
    Ok(title)
}

#[component]
fn TodoApp() -> impl IntoView {
    let (refresh, set_refresh) = signal(0_u64);

    let items = Resource::new(
        move || refresh.get(),
        |_| async move { load_todos().await },
    );

    let create = Action::new(move |title: &String| {
        let title = title.clone();
        let set_refresh = set_refresh;

        async move {
            let saved = create_todo(title).await?;
            set_refresh.update(|version| *version += 1);
            Ok(saved)
        }
    });

    provide_context(Todos {
        items,
        create,
        refresh: set_refresh,
    });

    view! {
        <Div>
            <TodoList />
            <TodoActions />
        </Div>
    }
}

#[component]
fn TodoList() -> impl IntoView {
    let todos = expect_context::<Todos>();

    dynamic(move || match todos.items.get_untracked() {
        Some(Ok(items)) => div(items.into_iter().map(text).collect::<Vec<_>>()),
        Some(Err(error)) => text(format!("Load failed: {error}")),
        None => text("Loading todos..."),
    })
}

#[component]
fn TodoActions() -> impl IntoView {
    let todos = expect_context::<Todos>();
    let create = todos.create.clone();
    let reload = todos.refresh;

    view! {
        <Div style={TuiStyle::new().display(Display::Flex)}>
            <Button on_press=move || {
                create.dispatch(String::from("Review async state"));
                AppControl::Continue
            }>
                "Create"
            </Button>
            <Button on_press=move || {
                reload.update(|version| *version += 1);
                AppControl::Continue
            }>
                "Reload"
            </Button>
        </Div>
    }
}
```

See `cargo run --example async_redraw` for a small async redraw example and
`cargo run --example async_crud` for resources, actions, context, stylesheets,
and app startup working together.

## Root-Scoped File System

`use_file_system()` canonicalizes a directory boundary and returns a cloneable
handle whose methods start blocking I/O outside the terminal event thread.
Call it directly inside any component that needs filesystem access rather than
passing a handle through the component tree. Relative paths resolve below that
root. Absolute paths and symlink
targets are accepted only when their canonical targets remain contained.
An exact leading `~` or `~/` expands to the current user's home directory for
both root initialization and every action path; named-user forms such as
`~alice` remain literal. Expanded paths still must remain within the configured
root.

```rust,no_run
use leptatui::prelude::*;

#[component]
fn ConfigDirButton() -> ViewResult<impl IntoView> {
    let files = use_file_system("~/.config")?;

    view! {
        <Button on_press=move || {
            // Ignoring the handle is valid for fire-and-forget work.
            files.create_dir("my-app/cache");
            AppControl::Continue
        }>
            "Create config directory"
        </Button>
    }
}

#[component]
fn ProjectsDirButton() -> ViewResult<impl IntoView> {
    let files = use_file_system("~/Projects")?;
    let latest_read = RwSignal::new(None::<FileOperation<String>>);

    view! {
        <Button on_press=move || {
            // Retain the already-started operation to render pending/value/version.
            latest_read.set(Some(files.read_file_as_string("README.md")));
            AppControl::Continue
        }>
            "Read project guide"
        </Button>
    }
}
```

Every operation method returns an already-dispatched `FileOperation<T>`, an
alias for `Action<(), std::io::Result<T>>`. Keep it to observe reactive state,
call `dispatch(())` to retry the same captured arguments, or ignore it when no
result is needed.

The API includes path resolution, metadata, deterministic directory listings,
binary and UTF-8 reads, recursive directory creation, truncating, appending,
and atomic writes, non-overwriting copy and rename, and file or recursive
directory deletion. `rename()` supports files, directories, and symbolic links.
Missing roots remain an error unless `FileSystemOptions::create_root(true)` is
selected, and recursive deletion can never target the scoped root itself.

Run `cargo run --example file_system_showcase` for a guided demonstration that
executes every operation in dependency order, retains reactive results, and
offers a separate expected-failure tour.

Useful follow-up capabilities include filesystem watching for automatic UI
reloads, recursive walking and glob matching, streaming and configurable open
options for large files, permission and symlink management, and platform
config/cache/data-directory discovery.

## Examples

Run the Hello World example:

```sh
cargo run --example hello_world
```

Press `q` to exit.

Run the responsive flexbox example:

```sh
cargo run --example responsive_flex
```

Resize the terminal to 60 columns or fewer to stack the navigation, content,
and sidebar.

Run the responsive grid dashboard:

```sh
cargo run --example responsive_grid
```

Resize the terminal to 60 columns or fewer to replace the two-column dashboard
with a single stacked column.

Run the interactive counter:

```sh
cargo run --example counter
```

Run the multi-page routing demo:

```sh
cargo run --example multi_page_demo
```

Run the fallible-view and panic-cleanup showcase:

```sh
cargo run --example error_handling
```

Run the standard component showcase:

```sh
cargo run --example standard_library_showcase
```

Run the semantic document component showcase:

```sh
cargo run --example document_showcase
```

Run the full-screen reader with its bundled showcase:

```sh
cargo run --example markdown_reader
```

Read any local UTF-8 Markdown file instead:

```sh
cargo run --example markdown_reader -- README.md
```

Run the reader against its bundled semantic and syntax-highlighting fixture:

```sh
cargo run --example markdown_reader -- crates/leptatui/examples/assets/markdown_showcase.md
```

See `crates/leptatui/examples/README.md` for controls.

## Commands

```sh
cargo metadata --format-version 1
cargo check --workspace
cargo check --workspace --examples
cargo test --workspace
```
