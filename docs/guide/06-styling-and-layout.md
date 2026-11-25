# 6. Styling and layout

Prerequisites: [Chapter 5: Reusable components](05-reusable-components.md).

Give the notes app a readable, responsive layout. Make the following two edits
in your notes project's `src/main.rs`, continuing from chapter 5.

## Add the stylesheet to `NotesApp`

Find `fn NotesApp() -> impl IntoView`. Keep its quit handler, signal definitions,
and `provide_context(Draft { title, body, status });` call. Immediately after
that `provide_context` call, insert the following `stylesheet!` block, before
the existing `view!` expression:

```rust
stylesheet! {
    H1 { fg: Color::LightCyan; }

    Button:focus { fg: Color::Yellow; }

    .notes-shell {
        padding: TuiSpacing::uniform(1);

        &__actions {
            display: Display::Flex;
            gap: Axes::all(Length::cells(1.0));
            @media (max-width: 60) {
                flex_direction: FlexDirection::Column;
            }
        }
    }
}
```

## Replace the shell view

Still inside `NotesApp`, replace its existing `view! { <Panel> ... </Panel> }`
expression with the following. This keeps `Panel`, `NoteHeading`, and
`NoteEditor` from chapter 5, wraps the shell in a styled `Div`, and adds a row
of Reset and Quit buttons:

```rust
view! {
    <Panel>
        <Div class="notes-shell">
            <NoteHeading text="My notes" />
            <Div class="notes-shell__actions">
                <Button on_press=move || {
                    title.set(String::from("First note"));
                    body.set(String::from("Learn to build a terminal application."));
                    status.set(String::from("Reset in memory"));
                }>
                    "Reset draft"
                </Button>
                <Button on_press=|| AppControl::Exit>"Quit"</Button>
            </Div>
            <NoteEditor />
        </Div>
    </Panel>
}
```

The nested `&__actions` selector expands to `.notes-shell__actions`, matching
that inner `Div`. Leave the `Draft`, `Panel`, `NoteHeading`, and `NoteEditor`
definitions and `main` function as they were in chapter 5. Run `cargo run` in
your notes project: the buttons should sit beside each other in a wide terminal
and stack when the terminal is 60 columns wide or narrower.

A stylesheet registered during component setup applies to that component's
subtree, including descendants. Built-in defaults remain available underneath
your rules. A helper containing `stylesheet!` must also run during setup.

## Work in terminal cells

One cell is one terminal character position. `TuiSpacing::uniform(1)` adds
one cell of padding on each side. `Length::cells`, `Length::percent`, and
viewport units (`vw`, `vh`, `vmin`, `vmax`) express typed sizes. `Dimension`
also supports automatic sizing.

The width and grid snippets below are optional experiments, rather than
additional edits to the notes app. To try either snippet, temporarily replace
`NotesApp`'s final `view!` expression with the entire snippet, including its
`stylesheet!` declaration. Keep the existing setup and stylesheet above it.
Restore the shell view from the previous section before continuing to chapter 7.

This example makes a container fill the available width:

```rust
stylesheet! {
    .full-width {
        size: LayoutSize::new(
            Dimension::from(Length::percent(100.0)),
            Dimension::Auto,
        );
    }
}
view! { <Div class="full-width"><Text>"Full width"</Text></Div> }
```

`Div` defaults to vertical block flow. Flexbox arranges direct children along
an axis; use `FlexDirection::Column` to stack a row when the terminal narrows.
Grid is useful for aligned dashboard rows and columns:

```rust
stylesheet! {
    .notes-grid {
        display: Display::Grid;
        grid_template_columns: vec![
            GridTemplateTrack::from(GridTrackSize::from(Length::cells(20.0))),
            GridTemplateTrack::from(GridTrackSize::from(Fraction::new(1.0))),
        ];
    }
}
view! {
    <Div class="notes-grid"><Text>"Notes"</Text><Text>"Selected note"</Text></Div>
}
```

## Scrolling and responsive behavior

Overflow is configured per axis with `Axes<Overflow>`. Automatic vertical
overflow is the default. `Auto` enables scrolling and a gutter only when
needed, `Scroll` always reserves a gutter, `Hidden` clips but allows scrolling,
`Clip` clips without scrolling, and `Visible` paints beyond the content area.
A constrained height makes an overflowing document into a useful scroll area.

Scroll with arrows, `j`/`k`, Page Up/Down, Ctrl+U/D, `gg`, `G`, or the wheel.
Focused editable controls may consume those keys for editing. Focus traversal
also scrolls controls into view. Media queries use terminal viewport dimensions
in cells and are reevaluated when the terminal resizes.

Run `responsive_flex`, `responsive_grid`, `block_layout`, or `nested_overflow`
with `cargo run --example NAME` from a Leptatui repository checkout. These
examples belong to the library repository, rather than your notes project.

For min/max sizing, alignment, grid placement, positioning, sticky/fixed boxes,
stacking, retained geometry, and terminal-specific limitations, use the
[layout reference](../reference/layout.md). For specificity, inheritance,
inline overrides, and `!important`, use the
[stylesheet reference](../reference/stylesheets.md).

## Complete code

Replace `src/main.rs` with the following. Use the Cargo manifest from
[chapter 1](01-getting-started.md) and run `cargo run` in your application
project. Ctrl+Q quits.

```rust
use leptatui::prelude::*;

#[derive(Clone, Copy)]
struct Draft {
    title: RwSignal<String>,
    body: RwSignal<String>,
    status: RwSignal<String>,
}

#[component]
fn NoteHeading(#[prop(into)] text: String) -> impl IntoView {
    view! { <H1>{text}</H1> }
}

#[component]
fn Panel(children: Children) -> impl IntoView {
    view! { <Block><Div>{children}</Div></Block> }
}

#[component]
fn NoteEditor() -> impl IntoView {
    let Draft { title, body, status } = expect_context::<Draft>();

    view! {
        <Div>
            <Form
                on_submit=move || {
                    status.set(String::from("Submitted in memory"));
                }
                on_cancel=move || {
                    status.set(String::from("Canceled; draft kept"));
                }
            >
                <Text>"Title"</Text>
                <Input value=title placeholder="Title"
                    on_input=move || {
                        status.set(String::from("Editing"));
                    }
                />
                <Text>"Body"</Text>
                <TextArea value=body placeholder="Write a note"
                    on_input=move || {
                        status.set(String::from("Editing"));
                    }
                />
            </Form>
            <Text>{status}</Text>
            <Text>"Tab focuses. i edits. Esc returns to normal mode. Enter submits in normal mode. Ctrl+Q quits."</Text>
        </Div>
    }
}

#[component]
fn NotesApp() -> impl IntoView {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return KeyControl::Exit;
        }
        KeyControl::Pass
    });
    let title = RwSignal::new(String::from("First note"));
    let body = RwSignal::new(String::from("Learn to build a terminal application."));
    let status = RwSignal::new(String::from("Editing"));

    provide_context(Draft { title, body, status });

    stylesheet! {
        H1 { fg: Color::LightCyan; }
        Button:focus { fg: Color::Yellow; }

        .notes-shell {
            padding: TuiSpacing::uniform(1);
            &__actions {
                display: Display::Flex;
                gap: Axes::all(Length::cells(1.0));
                @media (max-width: 60) {
                    flex_direction: FlexDirection::Column;
                }
            }
        }
    }
    view! {
        <Panel>
            <Div class="notes-shell">
                <NoteHeading text="My notes" />
                <Div class="notes-shell__actions">
                    <Button on_press=move || {
                        title.set(String::from("First note"));
                        body.set(String::from("Learn to build a terminal application."));
                        status.set(String::from("Reset in memory"));
                    }>
                        "Reset draft"
                    </Button>
                    <Button on_press=|| AppControl::Exit>"Quit"</Button>
                </Div>
                <NoteEditor />
            </Div>
        </Panel>
    }
}

#[tokio::main]
async fn main() -> leptatui::app::Result<()> {
    App::new(view! { <NotesApp /> }).run().await
}
```

[Previous: Reusable components](05-reusable-components.md) · [Documentation index](../README.md) · [Next: Pages and navigation](07-pages-and-navigation.md)
