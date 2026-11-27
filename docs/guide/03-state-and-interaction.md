# 3. State and interaction

Continue from [chapter 2: Building a screen](02-building-a-screen.md).

The note always shows the same text. Let's add a counter to see how a
button press changes what Leptatui draws.

A signal is a value that remembers who
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

Run `cargo run`. Press Tab to focus Increment, then Enter or Space to increase the count.
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

If you build the label with `let label = format!("Count: {}", count.get());`
before `view!`, it captures the starting count and keeps showing `Count: 0`.
In the example above, `count.get()` is inside `view!`, so Leptatui can read
the latest count and format the label again whenever it changes.

To show different content depending on the count, put the `if` inside a
`move ||` closure in `view!`. For example, you could add this line below the
count label. Leptatui runs the closure again when `count` changes, so pressing
Increment replaces the first message and pressing Reset brings it back.

```rust
<Text>{move || if count.get() == 0 { "At zero" } else { "Away from zero" }}</Text>
```

## Keyboard handlers and callbacks

Think about what happens when you press Tab. The keyboard handler you wrote
in chapter 1 sees that Tab doesn't match Ctrl+Q and returns `KeyControl::Pass`.
That gives Leptatui's built-in focus handling a chance to move you to the next
button. Every `use_key_event` handler returns a `KeyControl` value to say what
should happen next.

- `KeyControl::Pass` lets other handlers and built-in behavior try the key.
- `KeyControl::Handled` consumes the key and keeps the app running.
- `KeyControl::Exit` quits the app, as Ctrl+Q does here.

Components nested inside yours get the first chance to handle the key. If
they pass it along, your handler runs before Leptatui's built-in focus and
activation behavior. Use `Pass` for keys outside your shortcut so controls
can still respond to their usual keys.

Now press Enter while Increment is focused. Leptatui activates the button
and calls its `on_press` callback, which updates the count and returns `()`.
That keeps the app running, just like returning `AppControl::Continue`.
If you add a Quit button, its callback can return `AppControl::Exit` instead.
Input, text area, and form callbacks follow the same convention.

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
