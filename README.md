# Leptatui

Leptatui is an experimental Rust terminal UI runtime that combines Leptos
reactive primitives with Ratatui rendering and Crossterm event handling. It
provides a small view tree, component contract, styling helpers, and procedural
macros for building interactive terminal applications.

## Workspace Layout

- `crates/leptatui`: Public runtime crate with app, component, view, context,
  style, and prelude APIs.
- `crates/leptatui-macros`: Internal proc-macro crate that implements
  `#[component]`, `view!`, and `stylesheet!`.
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
`App::new(root).run().await`.

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
async fn main() -> Result<()> {
    App::new(Greeting::new()).run().await
}
```

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
            <Column>
                <Text>{title}</Text>
                {column(children())}
            </Column>
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

Leptatui's standard view set includes layout containers (`Block`, `Row`,
`Column`), plain and semantic text, nested lists, responsive tables,
syntax-highlighted code blocks, CommonMark documents, buttons, controlled form
controls, images, and progress bars.
All are available through `leptatui::prelude::*` as builders and through
PascalCase `view!` tags.

`View` is an open, object-safe trait. Built-in builders retain concrete return
types such as `TextView`, `LayoutView`, `InputView`, and `TextAreaView`, so their
type-specific configuration remains available without pattern matching.
Containers accept homogeneous collections or heterogeneous tuples through
`IntoViews`; type erasure occurs only when children enter the tree as
`AnyView` values. Application-defined views can implement `View` directly,
optionally implement `StyledView` or `ContainerView`, and use
`RenderCtx::resolve_style` to participate in the normal stylesheet cascade.
Custom selector names are created with `ViewType::new("Name")`, and the same
name can be used as a `stylesheet!` type selector.

`Link` opens URLs and local paths with the operating system's configured
handler. Use `link("Project site", "https://example.com")` or
`<Link href="https://example.com">"Project site"</Link>` anywhere in a view.
Links are underlined by default; use Tab and Shift+Tab to move focus and Enter
or Space to activate the focused link. Moving the pointer over a link focuses
it, and left-clicking activates it. Empty and `#fragment` targets remain visible
but are intentionally not focusable.

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

let document = column((
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
        .syntax_theme(SyntaxTheme::Dark)
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
        <Column>
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
                syntax_theme={SyntaxTheme::Dark}
                line_numbers=true
            >"fn main() {}"</CodeBlock>
        </Column>
    }
}
```

Headings, paragraphs, list items, table cells, and code lines wrap to the
available width and contribute their wrapped height to parent layouts. Code
blocks do not scroll horizontally. A recognized `language` selects a bundled
syntax grammar; an unknown language keeps the source readable as plain text.
`SyntaxTheme::Dark` is the default, and line numbers are disabled unless
requested. Table cells accept inline Ratatui text rather than nested block
views in the v1 API.

### Markdown Readers

Use `markdown` for default in-memory conversion or `markdown_with_options` to
configure every parsed code block:

```rust
use leptatui::prelude::*;

let source = "# Guide\n\n```rust\nfn main() {}\n```";
let default_document = markdown(source);
let highlighted_document = markdown_with_options(
    source,
    MarkdownOptions::default()
        .syntax_theme(SyntaxTheme::Light)
        .line_numbers(true),
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
    MarkdownOptions::default()
        .syntax_theme(SyntaxTheme::Dark)
        .line_numbers(true),
);
let tagged_file = view! {
    <Markdown
        src="README.md"
        syntax_theme={SyntaxTheme::Dark}
        line_numbers=true
    />
};
```

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
        <Column>
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
        </Column>
    }
}
```

Styles live with components. Put `stylesheet!` inside a `#[component]` body to
register those rules for that component subtree, including descendant
components. The same macro still returns a `Stylesheet` value for direct
construction and tests. It supports flat terminal selectors and nested rules
that lower into explicit descendant selectors. Use `&:focus` inside a nested
rule to combine focus with the current terminal selector. Matching rules use
CSS-style specificity and source order; inline styles override normal
stylesheet declarations, and stylesheet declarations marked `!important`
override normal inline styles.

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

Stylesheets also support top-level media query blocks for responsive terminal
UIs. Width and height values are terminal-cell counts from the root viewport,
and `direction` can switch Row/Column layout at a breakpoint.

```rust
#[component]
fn ResponsivePanel() -> impl IntoView {
    stylesheet! {
        .panel => { padding: TuiSpacing::uniform(1) }

        @media (max-width: 80) {
            .panel => { padding: TuiSpacing::ZERO }
            .actions => { direction: LayoutDirection::Column }
            Text => { fg: Color::Yellow }
        }

        @media (min-width: 81) and (min-height: 24) {
            Button:focus => { bg: Color::Yellow }
        }
    }

    view! {
        <Block class="panel">
            <Column>
                <Text>"Devtools"</Text>
                <Row class="actions">
                    <Button>"Inspect"</Button>
                    <Button>"Quit"</Button>
                </Row>
            </Column>
        </Block>
    }
}
```

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
        <Row>
            <Button class="submit">"Submit"</Button>
            <Button class="quit">"Quit"</Button>
        </Row>
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
            <Column>
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
            </Column>
        </Block>
    }
}
```

See `cargo run --example theme_switcher` for the complete light/dark theme
switcher.

Multi-page apps use ordinary Leptatui components with route state stored in
typed context. Keep `main` focused on startup, build shared state in the root
component, provide route and app-wide context there, and let page components
consume the values they need. The active page is usually a small enum, updated
through the route write signal returned by `provide_route()` or by
`use_navigate()` in descendants. The root then switches pages from a dynamic
`view!` child.

Use props for required parent-to-child inputs and local component
configuration. Use context for app-wide state that many routes or deeply nested
descendants need to read or update, such as the active route, theme variables,
or persisted settings. Keep shared state owned by the root when navigation
should not reset it; add explicit reset behavior when a route change should
clear shared state. Descendant pages can read required context with
`expect_context()` or optional context with `use_context()`.

```rust
use leptatui::prelude::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Page {
    Home,
    Counter,
    Settings,
}

#[component]
fn Nav() -> impl IntoView {
    let navigate = use_navigate::<Page>();

    view! {
        <Row>
            <Button on_press=move || {
                navigate.update(|route| *route = Page::Home);
                AppControl::Continue
            }>"Home"</Button>
            <Button on_press=move || {
                navigate.update(|route| *route = Page::Counter);
                AppControl::Continue
            }>"Counter"</Button>
            <Button on_press=move || {
                navigate.update(|route| *route = Page::Settings);
                AppControl::Continue
            }>"Settings"</Button>
        </Row>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    let counter = expect_context::<RwSignal<i32>>();

    view! {
        <Column>
            <Text>"Home"</Text>
            {move || {
                view! {
                    <Text>{format!("Count: {}", counter.get_untracked())}</Text>
                }
            }}
        </Column>
    }
}

#[component]
fn CounterPage() -> impl IntoView {
    let counter = expect_context::<RwSignal<i32>>();

    view! {
        <Column>
            <Text>"Counter"</Text>
            <Button on_press=move || {
                counter.update(|count| *count += 1);
                AppControl::Continue
            }>
                "Increment"
            </Button>
        </Column>
    }
}

#[component]
fn SettingsPage() -> impl IntoView {
    let route = use_route::<Page>();

    view! {
        {move || {
            view! {
                <Text>{format!("Current page: {:?}", route.get_untracked())}</Text>
            }
        }}
    }
}

#[component]
fn Root() -> impl IntoView {
    let counter = RwSignal::new(0);
    let route_state = provide_route(Page::Home);
    let route = route_state.route();

    provide_context(counter);

    view! {
        <Column>
            <Nav />
            {move || match route.get_untracked() {
                Page::Home => view! { <HomePage /> },
                Page::Counter => view! { <CounterPage /> },
                Page::Settings => view! { <SettingsPage /> },
            }}
        </Column>
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let root = Root::new();
    App::new(root).run().await
}
```

See `crates/leptatui/examples/multi_page_demo.rs` or run
`cargo run --example multi_page_demo` for a complete routing and context
example with shared counter and theme state.

## Resources And Actions

Use `create_resource` for asynchronous reads keyed by reactive source state and
`create_action` for POST-like mutations triggered by buttons or key handlers.
Both helpers publish signal-backed state and request redraws when pending or
completed work changes, so components can render loading, success, and error
states from ordinary dynamic views.

Resources ignore stale completions from older source keys. Actions keep the
latest dispatched input and ignore stale completions from older dispatches. A
common pattern is to keep a refresh signal in root context, key the resource
from that signal, and increment it after a successful action.

```rust
use leptatui::prelude::*;

#[derive(Clone)]
struct Todos {
    items: Resource<Vec<String>, String>,
    create: Action<String, String, String>,
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

    let items = create_resource(
        move || refresh.get(),
        |_| async move { load_todos().await },
    );

    let create = create_action(move |title: String| {
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
        <Column>
            <TodoList />
            <TodoActions />
        </Column>
    }
}

#[component]
fn TodoList() -> impl IntoView {
    let todos = expect_context::<Todos>();

    dynamic(move || match todos.items.get_untracked() {
        ResourceState::Pending => text("Loading todos..."),
        ResourceState::Ready(items) => column(items.into_iter().map(text).collect::<Vec<_>>()),
        ResourceState::Error(error) => text(format!("Load failed: {error}")),
    })
}

#[component]
fn TodoActions() -> impl IntoView {
    let todos = expect_context::<Todos>();
    let create = todos.create.clone();
    let reload = todos.refresh;

    view! {
        <Row>
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
        </Row>
    }
}
```

See `cargo run --example async_redraw` for a small async redraw example and
`cargo run --example async_crud` for resources, actions, context, stylesheets,
and app startup working together.

## Examples

Run the Hello World example:

```sh
cargo run --example hello_world
```

Press `q` to exit.

Run the interactive counter:

```sh
cargo run --example counter
```

Run the multi-page routing demo:

```sh
cargo run --example multi_page_demo
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
