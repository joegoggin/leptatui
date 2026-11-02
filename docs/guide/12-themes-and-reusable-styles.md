# 12. Themes and reusable styles

Prerequisites: [Chapter 11: Documents and media](11-documents-and-media.md).

Make the app's appearance reusable. Keep literal palette colors in a theme
model and use named variables in component styles.

Inside `NotesApp`, before its stylesheet and view, provide theme variables:

```rust
let theme = RwSignal::new(
    ThemeVariables::new()
        .color("text", Color::White)
        .color("surface", Color::Black),
);
provide_context(theme.read_only());
stylesheet! {
    .notes-shell => {
        fg: theme_color("text"),
        bg: theme_color("surface")
    }
}
```

A button can call `theme.set(ThemeVariables::new().color("text",
Color::Black).color("surface", Color::White))` and return
`AppControl::Continue`. Styles resolve variables while rendering, so descendants
repaint without changing their component code. For fixed themes, provide a
plain `ThemeVariables` instead of its read signal. The complete toggle example
is `cargo run --example theme_switcher`.

## Variables, mixins, and imports

Local `$variables` avoid repeating values; `@mixin` defines reusable
declarations. Helpers returning only variables and mixins produce a
`StyleModule`, which another stylesheet imports with `@use`:

```rust
fn note_styles() -> StyleModule {
    stylesheet! {
        $accent: Color::LightCyan;
        @mixin heading {
            fg: $accent,
            modifier: Modifier::BOLD
        }
    }
}

#[component]
fn StyledTitle() -> impl IntoView {
    stylesheet! {
        @use note_styles as notes;
        H1 => { @include notes.heading }
    }
    view! { <H1>"My notes"</H1> }
}
```

Nested rules express descendant selectors. `&:focus`, `&:active`, `&:insert`,
`&:visual`, and `&:visited` combine supported states with the containing
selector. Beneath a class, `&__element` and `&--modifier` concatenate class
names, useful for component naming conventions. Specificity and source order
choose between rules; inline styles override normal stylesheet rules, while
stylesheet `!important` overrides normal inline declarations. Inheritable
values flow through the tree; do not assume every layout property inherits.

As components grow, place a component and its style helper together in a
module directory. A helper in `style.rs` should invoke `stylesheet!` during
component setup and end it with a semicolon when returning `()`. Routed pages
can use the same pattern. The [stylesheet reference](../reference/stylesheets.md)
contains the module layout and exact cascade examples.

Run `stylesheet_imports`, `style_cascade_showcase`, or
`default_styles_showcase` with `cargo run --example NAME`.

[Previous: Documents and media](11-documents-and-media.md) · [Documentation index](../README.md) · [Next: Extending Leptatui](13-extending-leptatui.md)
