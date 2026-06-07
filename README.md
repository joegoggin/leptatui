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

```rust
use leptatui::prelude::*;

#[component]
fn Greeting() -> Node {
    view! { <Text>"Hello from Leptatui"</Text> }
}

#[tokio::main]
async fn main() -> Result<()> {
    App::new(Greeting::new()).run().await
}
```

Styles can be built directly with `Stylesheet::new().rule(...)` or with the
`stylesheet!` macro.

## Examples

Run the smoke example:

```sh
cargo run --example app_smoke
```

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
