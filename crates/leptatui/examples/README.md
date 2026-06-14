# Leptatui Examples

Runnable examples for the public `leptatui` crate live in this directory.
Each example includes responsive stylesheet rules for terminals at or below 60
columns, which is useful when running over SSH from a phone.

## Hello World

Run the Hello World example:

```sh
cargo run --example hello_world
```

Press `q` to exit.

## Counter

Run the styled counter example:

```sh
cargo run --example counter
```

Press `+`/`=` to increment, `-` to decrement, `r` to reset, and `q` to quit.

## Theme Switcher

Run the context-backed light/dark theme example:

```sh
cargo run --example theme_switcher
```

Use Tab and Shift+Tab to move focus. Activate Toggle theme with Enter or Space
to switch the active theme variables at runtime.

## Style Cascade Showcase

Run the style cascade mechanics showcase:

```sh
cargo run --example style_cascade_showcase
```

This demonstrates CSS-like specificity, source order, descendant selectors,
component boundaries, inline styles, `!important`, inheritance, and focus
styling. Use Tab and Shift+Tab to move focus. Activate buttons with Enter or
Space, and press `q` to quit.

## Stylesheet Imports

Run the imported variables and mixins example:

```sh
cargo run --example stylesheet_imports
```

This demonstrates `@use`, namespaced variables, imported mixins, and local
mixin composition. Use Tab and Shift+Tab to move focus. Activate buttons with
Enter or Space, and press `q` to quit.
