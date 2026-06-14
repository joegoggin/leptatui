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

## Usage Shape

Application code normally imports `leptatui::prelude::*`, defines a root
component, builds a view tree with either builders or `view!`, and runs it with
`App::new(root).run().await`.

Generated `#[component]` bodies run once when `new()` creates the component,
under a stored Leptos owner. Create signals directly in the component body, and
read them from dynamic views or event handlers when values need to update.
Buttons support Tab/Shift+Tab focus movement and Enter/Space activation by
default. Register handlers with `use_key_event()` inside a component body for
custom key maps and overrides.

```rust
use leptatui::prelude::*;

#[component]
fn Greeting() -> View {
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
fn Label(#[prop(into)] text: String) -> View {
    view! { <Text>{text}</Text> }
}

#[component]
fn Panel(#[prop(into)] title: String, children: Children) -> View {
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
fn Panel() -> View {
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
fn ResponsivePanel() -> View {
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
fn MixedPanel() -> View {
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
fn Actions() -> View {
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
fn ThemedPanel() -> View {
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
fn ThemeLabel() -> View {
    let mode = expect_context::<ReadSignal<ThemeMode>>();

    dynamic(move || {
        view! { <Text>{format!("Theme: {:?}", mode.get_untracked())}</Text> }
    })
}

#[component]
fn ThemeRoot() -> View {
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

Route state can live in context the same way shared app state does. Provide a
small route enum near the root, navigate with the returned write signal or
`use_navigate()` in descendants, and branch inside a dynamic `view!` child.
Root-owned signals and context remain owned by the root component while the
visible page branch changes.

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Page {
    Home,
    Counter,
    Settings,
}

#[component]
fn Nav() -> View {
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
fn Root() -> View {
    let route_state = provide_route(Page::Home);
    let route = route_state.route();

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
```

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

See `crates/leptatui/examples/README.md` for controls.

## Commands

```sh
cargo metadata --format-version 1
cargo check --workspace
cargo check --workspace --examples
cargo test --workspace
```
