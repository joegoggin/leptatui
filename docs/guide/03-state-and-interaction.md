# 3. State and interaction

Prerequisites: [Chapter 2: Building a screen](02-building-a-screen.md).

Make the note respond to counter buttons. A signal is a value that remembers who
reads it, allowing the interface to update when that value changes.

In `src/app.rs`, immediately after the quit handler in `NotesApp`, add:

```rust
let count = RwSignal::new(0_i32);
```

Replace the view expression with:

```rust
view! {
    <Block>
        <Div>
            <Text>"First note"</Text>
            <Paragraph>"Learn to build a terminal application."</Paragraph>
            <Text>{format!("Count: {}", count.get())}</Text>
            <Button on_press=move || {
                count.update(|value| *value += 1);
            }>"Increment"</Button>
            <Button on_press=move || {
                count.update(|value| *value -= 1);
            }>"Decrement"</Button>
            <Button on_press=move || {
                count.set(0);
            }>"Reset"</Button>
            <Text>"Tab focuses buttons; Enter activates. Ctrl+Q quits."</Text>
        </Div>
    </Block>
}
```

Press Tab to focus Increment, then Enter or Space to increase the count.
Tab again to reach Decrement, which decreases the count, then Reset, which
returns it to zero. Shift+Tab moves focus backward. Pointer movement can focus
controls; left-click activates a button or link.

`update()` gives the callback a mutable reference to the current value, so it
fits changes that depend on that value. `set()` replaces the value directly,
as Reset does with `count.set(0)`. The counter uses `i32` so Decrement can
take it below zero.

## Automatic tracking

The component body runs once per mounted component instance. Expressions inside
`view!`, including text, attributes, and data props, automatically track the
signals they read. Changing `count` updates the count without rerunning
component setup or requiring a closure around the view.

A value computed before `view!` is still a snapshot. Keep computations that
should update inside the view expression. For conditional UI, use a tracked
child closure inside `view!` so signal changes update the displayed branch.

## Keyboard handlers and callbacks

The chapter 1 handler returns `KeyControl::Pass` for keys it does not handle.
`Handled` stops propagation while keeping the app running; `Exit` exits.
Registered handlers run after descendant components' handlers and before
built-in focus and activation handling. Handle only the keys you intend to
claim, particularly around editable controls.

Button, input, text area, and form callbacks can return `()` (including by
omitting a return expression) to keep the app running. Return
`AppControl::Exit` to quit; explicit `AppControl::Continue` also works.
The two control enums serve different jobs: key propagation and app lifetime.
Raw keyboard handlers still return `KeyControl` explicitly.

For a separate example with custom shortcuts, run
`cargo run --example counter` in the library checkout.

## Complete files for this chapter

Create or replace only the files listed below; retain all other files from
the previous chapter. Keep the Cargo manifest from [chapter 1](01-getting-started.md).

Run `cargo run` in your application project. Ctrl+Q quits.

### `src/app.rs`

```rust
use leptatui::prelude::*;

#[component]
pub(crate) fn NotesApp() -> impl IntoView {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return KeyControl::Exit;
        }
        KeyControl::Pass
    });

    let count = RwSignal::new(0_i32);

    view! {
        <Block>
            <Div>
                <Text>"First note"</Text>
                <Paragraph>"Learn to build a terminal application."</Paragraph>
                <Text>{format!("Count: {}", count.get())}</Text>
                <Button on_press=move || {
                    count.update(|value| *value += 1);
                }>"Increment"</Button>
                <Button on_press=move || {
                    count.update(|value| *value -= 1);
                }>"Decrement"</Button>
                <Button on_press=move || {
                    count.set(0);
                }>"Reset"</Button>
                <Text>"Tab focuses buttons; Enter activates. Ctrl+Q quits."</Text>
            </Div>
        </Block>
    }
}
```

[Previous: Building a screen](02-building-a-screen.md) · [Documentation index](../README.md) · [Next: Editing notes](04-editing-notes.md)
