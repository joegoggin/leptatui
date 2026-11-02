# 3. State and interaction

Prerequisites: [Chapter 2: Building a screen](02-building-a-screen.md).

Make the note respond to a button. A signal is a value that remembers who
reads it, allowing the interface to update when that value changes.

Immediately after the quit handler in `NotesApp`, add:

```rust
let revisions = RwSignal::new(0_u32);
```

Replace the view expression with:

```rust
view! {
    <Block>
        <Div>
            <Text>"First note"</Text>
            <Paragraph>"Learn to build a terminal application."</Paragraph>
            <Text>{move || format!("Revisions: {}", revisions.get())}</Text>
            <Button on_press=move || {
                revisions.update(|value| *value += 1);
                AppControl::Continue
            }>"Revise"</Button>
            <Text>"Tab focuses the button; Enter revises. Ctrl+Q quits."</Text>
        </Div>
    </Block>
}
```

Press Tab to focus the button, then Enter or Space. The revision count should
increase immediately. Pointer movement can focus controls; left-click activates
a button or link. Shift+Tab moves focus backward.

## Why the closure matters

The component body runs once per mounted component instance. A plain
`format!("{}", revisions.get())` there captures a snapshot. The `move || ...`
closure runs inside a tracked dynamic boundary: `.get()` subscribes it to the
signal and a change rebuilds that part of the interface. `.with()` borrows a
tracked value; `.set()` replaces it; `.update()` changes it in place.

Supported readable signals containing text can also be passed directly as
children. `Memo` derives a cached value from other signals. `Effect::new`
runs reactive side effects; use it for integration work, rather than copying
state that can be derived directly. Untracked reads (`get_untracked` and
`with_untracked`) deliberately avoid subscribing the current computation.

The runtime wakes for reactive changes, including async completions, rather
than continuously redrawing an idle interface.

## Keyboard handlers and callbacks

The chapter 1 handler returns `KeyControl::Pass` for keys it does not handle.
`Handled` stops propagation while keeping the app running; `Exit` exits.
Registered handlers run after descendant components' handlers and before
built-in focus and activation handling. Handle only the keys you intend to
claim, particularly around editable controls.

Button and form callbacks return `AppControl::Continue` or `AppControl::Exit`.
The two control enums serve different jobs: key propagation and app lifetime.

For a separate example with custom shortcuts, run
`cargo run --example counter` in the library checkout.

[Previous: Building a screen](02-building-a-screen.md) · [Documentation index](../README.md) · [Next: Editing notes](04-editing-notes.md)
