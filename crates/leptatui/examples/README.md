# Leptatui Examples

Runnable examples for the public `leptatui` crate live in this directory.

## Smoke App

Run the minimal app runner smoke example:

```sh
cargo run --example app_smoke
```

Use Tab and Shift+Tab to focus Quit, then press Enter or Space to exit.

## Counter

Run the styled counter example:

```sh
cargo run --example counter
```

Use Tab and Shift+Tab to move focus between buttons. Press Enter or Space to
activate the focused button, and activate Quit to exit.

## Theme Switcher

Run the context-backed light/dark theme example:

```sh
cargo run --example theme_switcher
```

Use Tab and Shift+Tab to move focus. Activate Toggle theme with Enter or Space
to switch the active theme variables at runtime.
