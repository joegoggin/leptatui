# Stylesheet reference

Start with [Styling and layout](../guide/06-styling-and-layout.md) and [Themes and reusable styles](../guide/12-themes-and-reusable-styles.md).

## On this page

- [Rules and component scope](#rules-and-component-scope)
- [Modules, mixins, and themes](#modules-mixins-and-themes)

## Rules and component scope

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

## Modules, mixins, and themes

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
        view! { <Text>{format!("Theme: {:?}", mode.get())}</Text> }
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

[Documentation index](../README.md) · [Feature index](features.md)
