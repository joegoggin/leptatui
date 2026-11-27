# 6. Styling and layout

Continue from [chapter 5: Reusable components](05-reusable-components.md).

Your editor has a shell of its own. Let's style it and add buttons
that change layout as you resize the terminal.

## Co-locate styles with the shell

The shell owns its styles, so register them when `RootLayout` is set up.
The helper is synchronous and ends the `stylesheet!` invocation with a
semicolon so it returns `()`.

Create these empty files, including any directories they need.

- `src/layouts/root/style.rs`

Add the following to `src/layouts/root/mod.rs`.

```rust
mod style;
```

Fill in `src/layouts/root/style.rs` with these contents.

```rust
use leptatui::prelude::*;

pub(super) fn use_root_layout_styles() {
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
    };
}
```

Update `src/layouts/root/component.rs` with these contents.

```rust
use leptatui::prelude::*;

use super::components::NoteHeading;
use super::style::use_root_layout_styles;

#[component]
pub(crate) fn RootLayout(children: Children) -> impl IntoView {
    use_root_layout_styles();

    view! {
        <Block>
            <Div>
                <Div class="notes-shell">
                    <NoteHeading text="My notes" />
                    {children}
                </Div>
            </Div>
        </Block>
    }
}
```

In `src/app.rs`, replace the final `view!` in `NotesApp`.

```rust
view! {
    <RootLayout>
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
    </RootLayout>
}
```

The nested `&__actions` selector expands to `.notes-shell__actions`, matching
the action row in `app.rs`. The heading rule and focus rule apply throughout
the layout's subtree. Register styles in the component that owns that subtree;
placing Rust code in another file does not change stylesheet scope.

Run `cargo run`: the buttons should sit beside each other in a wide terminal
and stack when the terminal is 60 columns wide or narrower. Reset should
replace the draft and status; Quit should return to the shell.

## Work in terminal cells

One cell is one terminal character position. `TuiSpacing::uniform(1)` adds
one cell of padding on each side. `Length::cells`, `Length::percent`, and
viewport units (`vw`, `vh`, `vmin`, `vmax`) express typed sizes. `Dimension`
also supports automatic sizing.

The width and grid snippets below are optional experiments, rather than
additional edits to the notes app. To try either snippet, temporarily replace
`RootLayout`'s final `view!` expression in `src/layouts/root/component.rs` with
the entire snippet, including its `stylesheet!` declaration. Keep the existing
setup and call to `use_root_layout_styles()` before it. Restore the root layout
view from the complete files below before continuing to chapter 7.

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

## Complete files for this chapter

Create or replace only the files listed below; retain all other files from
the previous chapter. Keep the Cargo manifest from [chapter 1](01-getting-started.md).

The project now contains:

```text
└── src/
    ├── app.rs
    ├── components/
    │   ├── mod.rs
    │   └── note_editor.rs
    ├── draft.rs
    ├── layouts/
    │   ├── mod.rs
    │   └── root/
    │       ├── component.rs
    │       ├── components/
    │       │   ├── mod.rs
    │       │   └── note_heading.rs
    │       ├── mod.rs
    │       └── style.rs
    └── main.rs
```

Run `cargo run` in your application project. Ctrl+Q quits.

### `src/app.rs`

```rust
use leptatui::prelude::*;

use crate::components::NoteEditor;
use crate::draft::Draft;
use crate::layouts::RootLayout;

#[component]
pub(crate) fn NotesApp() -> impl IntoView {
    use_key_event(KeyEventKind::Press, |key| {
        if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return KeyControl::Exit;
        }
        KeyControl::Pass
    });
    let title = RwSignal::new(String::from("First note"));
    let body = RwSignal::new(String::from("Learn to build a terminal application."));
    let status = RwSignal::new(String::from("Editing"));

    provide_context(Draft {
        title,
        body,
        status,
    });

    view! {
        <RootLayout>
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
        </RootLayout>
    }
}
```

### `src/layouts/root/component.rs`

```rust
use leptatui::prelude::*;

use super::components::NoteHeading;
use super::style::use_root_layout_styles;

#[component]
pub(crate) fn RootLayout(children: Children) -> impl IntoView {
    use_root_layout_styles();

    view! {
        <Block>
            <Div>
                <Div class="notes-shell">
                    <NoteHeading text="My notes" />
                    {children}
                </Div>
            </Div>
        </Block>
    }
}
```

### `src/layouts/root/mod.rs`

```rust
mod component;
mod components;
mod style;

pub(crate) use component::RootLayout;
```

### `src/layouts/root/style.rs`

```rust
use leptatui::prelude::*;

pub(super) fn use_root_layout_styles() {
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
    };
}
```

[Previous: Reusable components](05-reusable-components.md) · [Documentation index](../README.md) · [Next: Pages and navigation](07-pages-and-navigation.md)
