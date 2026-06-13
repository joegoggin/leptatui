# Leptatui

Leptatui is an experimental Rust terminal UI runtime that combines Leptos
reactive primitives with Ratatui rendering and Crossterm event handling. It
provides a small node tree, component contract, styling helpers, and procedural
macros for building interactive terminal applications.

## Workspace Layout

- `crates/leptatui`: Public runtime crate with app, component, node, context,
  style, and prelude APIs.
- `crates/leptatui-macros`: Internal proc-macro crate that implements
  `#[component]`, `view!`, and `stylesheet!`.
- `crates/leptatui/examples`: Runnable examples for the public crate.

## Usage Shape

Application code normally imports `leptatui::prelude::*`, defines a root
component, builds a node tree with either builders or `view!`, and runs it with
`App::new(root).run().await`.

Generated `#[component]` bodies run once when `new()` creates the component,
under a stored Leptos owner. Create signals directly in the component body, and
read them from dynamic nodes or event handlers when values need to update.
Buttons support Tab/Shift+Tab focus movement and Enter/Space activation by
default. Register handlers with `use_key_event()` inside a component body for
custom key maps and overrides.

```rust
use leptatui::prelude::*;

#[component]
fn Greeting() -> Node {
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

Styles can be built directly with `Stylesheet::new().rule(...)` or with the
`stylesheet!` macro. The macro supports flat terminal selectors and nested
rules that lower into explicit descendant selectors. Use `&:focus` inside a
nested rule to combine focus with the current terminal selector.

```rust
let stylesheet = stylesheet! {
    .panel => {
        bg: Color::Black

        Text => { fg: Color::White }

        Button => {
            &:focus => { bg: Color::Yellow }
        }
    }
};
```

Reusable declaration groups can be declared with `@mixin` and expanded in rule
bodies with `@include`.

```rust
let stylesheet = stylesheet! {
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
};
```

Stylesheets can also reference runtime theme variables. Provide
`ThemeVariables` through Leptatui context before rendering descendants, then
use `theme_color("name")` in stylesheet declarations.

```rust
let stylesheet = stylesheet! {
    $text: theme_color("text");
    $surface: theme_color("surface");

    .panel => { fg: $text, bg: $surface }
};

provide_context(
    ThemeVariables::new()
        .color("text", Color::Black)
        .color("surface", Color::White),
);
```

See `cargo run --example theme_switcher` for a light/dark theme switcher.

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

See `crates/leptatui/examples/README.md` for controls.

## Commands

```sh
cargo metadata --format-version 1
cargo check --workspace
cargo check --workspace --examples
cargo test --workspace
```
