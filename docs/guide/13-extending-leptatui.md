# 13. Extending Leptatui

Prerequisites: [Chapter 12: Themes and reusable styles](12-themes-and-reusable-styles.md).

Use the extension interfaces when existing builders cannot express a custom
rendering need. This is an advanced, standalone lesson; the notes app can keep
using built-in views.

## A custom leaf view

`View` is an open, object-safe trait. A render-only leaf implements `render`,
`as_any`, and `as_any_mut`; other methods have defaults. Add the following
code above a component, then return `Rule` from it:

```rust
struct Rule;

impl View for Rule {
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> leptatui::Result<()> {
        text("----------------").render(ctx)
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
editable content and callbacks. Built-in builders implement these capabilities
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

[Previous: Themes and reusable styles](12-themes-and-reusable-styles.md) · [Documentation index](../README.md)
