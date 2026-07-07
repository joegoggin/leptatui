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

## Controlled Form

Run the controlled form example:

```sh
cargo run --example controlled_form
```

Use Tab and Shift+Tab to move focus between the input, text area, and submit
button. Editable controls start in Vim normal mode; press `i`, `a`, `I`,
or `A` to enter insert mode. Press Esc while a form field is in normal mode to
cancel the form. Input fields submit with Enter. Text areas insert a newline in
insert mode with Enter and submit with Ctrl+Enter. Press `q` to quit.

## Theme Switcher

Run the context-backed light/dark theme example:

```sh
cargo run --example theme_switcher
```

Use Tab and Shift+Tab to move focus. Activate Toggle theme with Enter or Space
to switch the active theme variables at runtime.

## Multi-Page Demo

Run the route-driven Home, Counter, and Settings demo:

```sh
cargo run --example multi_page_demo
```

Use `h`, `c`, and `s` to switch pages. The Counter page supports `+`, `-`, and
`r`; the Settings page supports `t` for theme switching. Press `q` to quit.
Counter and theme settings are shared through root context, so changes persist
while moving between pages.

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

## Async Redraw

Run the async redraw example:

```sh
cargo run --example async_redraw
```

The initial resource completion should redraw after its delay without key
input. Use `r` to reload, `a` to dispatch the async action, and `q` to quit.

## Async CRUD

Run the async CRUD-style mock API demo:

```sh
cargo run --example async_crud
```

The demo loads a mock ticket list, dispatches create/update actions, refetches
after successful mutations, and renders pending, success, and error states.
Use `n` to create, `u` to update the first ticket, `r` to reload, `l` to fail
the next list request, `e` to fail a mutation, and `q` to quit.
