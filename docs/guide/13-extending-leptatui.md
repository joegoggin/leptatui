# 13. Extending Leptatui

Let's build a horizontal rule as a custom view. Make a separate copy
of the [chapter 1 project](01-getting-started.md), including its Cargo manifest and both Rust
files, to follow this standalone lesson.

## A custom leaf view

`View` is an open, object-safe trait. A render-only leaf implements `render`,
`as_any`, and `as_any_mut`; other methods have defaults. Add the following
code in `src/views/rule.rs`, with its own prelude import. Make `Rule`
`pub(crate)`, re-export it from `src/views/mod.rs`, and declare `mod views;`
in `main.rs`:

```rust
use leptatui::prelude::*;

pub(crate) struct Rule;

impl View for Rule {
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> leptatui::Result<()> {
        view! { <Text>"----------------"</Text> }.render(ctx)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
```

Every `View` converts through `IntoView`, so `{Rule}` also works as a child in
`view!`. This example delegates drawing to `Text`; a real custom leaf can use
`RenderCtx::render_widget` to draw a Ratatui widget into its assigned area.
If you import Ratatui types directly, add a compatible direct dependency to
your application (the checkout currently uses Ratatui 0.30).

## Measurement and styling

Implement `View::measure` when the parent needs meaningful intrinsic width and
height. Its inputs describe known dimensions and available space, and it
returns `LayoutSize<f32>`. The default measurement does not inspect custom
content, so a production widget should measure what it renders.

To join the normal cascade, retain `StyleMetadata`, expose it through the view
trait, and use `RenderCtx::resolve_style` when drawing. `StyledView` provides
the styling interface and `ContainerView` exposes child configuration.
`TextualView` configures textual content; `EditableView` configures controlled
editable content and callbacks. Built-in views implement these capabilities
where applicable. `ViewType::new("MyWidget")` assigns a custom type selector that a stylesheet
can target. Read the built-in [Text implementation](../../crates/leptatui/src/view/content/text.rs)
and [view contract](../../crates/leptatui/src/view/core/contract.rs) for the
complete measurement, interaction, and reconciliation interfaces before
implementing an interactive container.

`LayoutGeometry` retains border, padding, content, viewport, and clipping
rectangles after layout. Use the [layout reference](../reference/layout.md)
for rounding, clipping, and containing-block behavior rather than inferring
positions from string lengths.

## Custom application roots

`App` accepts `AppRoot`, the lower-level rendering/event adapter. Ordinary
views are already adapted to it, so normal apps need only
`App::new(view).run().await`. Implement a custom root when you need control of
frame rendering or event dispatch; inspect the
[AppRoot contract](../../crates/leptatui/src/app/root.rs) and generated API
documentation for its current methods.

Generated-code hooks under `__private` are unsupported implementation details.
Leptatui does not provide a DOM renderer or general raw terminal session
customization API. For temporary terminal ownership, use the supported
[app handle/editor integration](10-files-and-editors.md).

To validate an extension, render it at narrow and wide sizes with Ratatui's
test backend, check measurement and clipping, and exercise any focus and event
behavior it adds. The [existing view tests](../../crates/leptatui/tests/view/support/render.rs)
show how the library tests rendering. Contributor checks live in
[Working on Leptatui](../contributing.md).

## Mount the custom view

Start from chapter 1's two-file application in this separate project. Keep
`main.rs` for startup and the Ctrl+Q handler in `app.rs`. Import
`crate::views::Rule` in `app.rs`, then replace `NotesApp`'s final view with:

```rust
view! { <Div><H1>"Custom leaf view"</H1>{Rule}<Text>"Ctrl+Q quits"</Text></Div> }
```

Create `src/views/mod.rs` with these contents.

```rust
mod rule;

pub(crate) use rule::Rule;
```

In `src/main.rs`, add `mod views;` below `mod app;`. In `src/app.rs`,
add `use crate::views::Rule;` below the prelude import.

Run `cargo run`. You should see a heading and a horizontal line of dashes;
Ctrl+Q exits. This small example adds no direct Ratatui dependency.

## Complete files for this chapter

These are all files for the separate extension project. Use chapter 1's
Cargo manifest.

The project now contains:

```text
└── src/
    ├── app.rs
    ├── main.rs
    └── views/
        ├── mod.rs
        └── rule.rs
```

Run `cargo run` in your application project. Ctrl+Q quits.

### `src/app.rs`

```rust
use leptatui::prelude::*;

use crate::views::Rule;

#[component]
pub(crate) fn NotesApp() -> impl IntoView {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return KeyControl::Exit;
        }
        KeyControl::Pass
    });

    view! { <Div><H1>"Custom leaf view"</H1>{Rule}<Text>"Ctrl+Q quits"</Text></Div> }
}
```

### `src/main.rs`

```rust
mod app;
mod views;

use leptatui::prelude::*;

use crate::app::NotesApp;

#[tokio::main]
async fn main() -> leptatui::app::Result<()> {
    App::new(view! { <NotesApp /> }).run().await
}
```

### `src/views/mod.rs`

```rust
mod rule;

pub(crate) use rule::Rule;
```

### `src/views/rule.rs`

```rust
use leptatui::prelude::*;

pub(crate) struct Rule;
impl View for Rule {
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> leptatui::Result<()> {
        view! { <Text>"----------------"</Text> }.render(ctx)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
```

[Previous: Themes and reusable styles](12-themes-and-reusable-styles.md) · [Documentation index](../README.md)
