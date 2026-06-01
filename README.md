
# Leptatui

## Workspace Layout

- `crates/leptatui`: public runtime crate.
- `crates/leptatui-macros`: internal proc-macro crate.
- `crates/leptatui/examples`: runnable examples for the public crate.

## Commands

```sh
cargo metadata --format-version 1
cargo check --workspace
cargo check --workspace --examples
