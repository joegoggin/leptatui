# 12. Themes and reusable styles

Continue from [chapter 11: Documents and media](11-documents-and-media.md).

The notes app uses a single palette. Let's add a theme toggle and
watch the same views pick up new colors.

The theme holds the palette colors. Component styles refer to those colors
by name, so one change can update multiple views.

Inside `NotesApp` in `src/app.rs`, after providing `Draft` and before its
`view!`, add:

```rust
let light = RwSignal::new(false);
let theme = RwSignal::new(
    ThemeVariables::new()
        .color("text", Color::White)
        .color("surface", Color::Black),
);
provide_context(theme.read_only());
```

In `src/layouts/root/style.rs`, remove `H1 { fg: Color::LightCyan; }`;
we will style the heading through a reusable mixin below. Add these declarations
inside the existing `.notes-shell` rule, keeping its layout rules:

```rust
fg: theme_color("text");
bg: theme_color("surface");
```

In `src/app.rs`, add the following button after Preview in the navigation `Div`:

```rust
<Button on_press=move || {
    light.update(|value| *value = !*value);
    let (text, surface) = if light.get_untracked() {
        (Color::Black, Color::White)
    } else {
        (Color::White, Color::Black)
    };
    theme.set(ThemeVariables::new().color("text", text).color("surface", surface));
}>"Toggle theme"</Button>
```

Styles resolve variables while rendering, so descendants repaint without
changing their component code. For fixed themes, provide a plain
`ThemeVariables` instead of its read signal.
The separate library example is `cargo run --example theme_switcher`.

## Variables, mixins, and imports

Local `$variables` avoid repeating values; `@mixin` defines reusable
declarations. Helpers returning only variables and mixins produce a
`StyleModule`, which another stylesheet imports with `@use`.

The heading now needs room for a stylesheet. Move
`src/layouts/root/components/note_heading.rs` to
`src/layouts/root/components/note_heading/mod.rs`. Move the file rather than
copying it so Rust has a single definition for the module. The parent module
still declares `mod note_heading;`, and the heading is still available.

Create these empty files, including any directories they need.

- `src/layouts/root/components/note_heading/component.rs`
- `src/layouts/root/components/note_heading/style.rs`

Add the following to `src/layouts/root/components/note_heading/mod.rs`.

```rust
mod component;
mod style;
```

Put `note_styles() -> StyleModule` and `use_note_heading_styles()` in the new
`style.rs`. The first helper defines the reusable accent variable and heading
mixin; the second imports it with `@use note_styles as notes` and applies
`@include notes.heading` to `H1`. Call `use_note_heading_styles()` during
`NoteHeading` setup in `component.rs`. Keep the `text` prop unchanged.
The complete files below include both helpers and the module facade.

Fill in `src/layouts/root/components/note_heading/style.rs` with these contents.

```rust
use leptatui::prelude::*;

fn note_styles() -> StyleModule {
    stylesheet! {
        $accent: Color::LightCyan;
        @mixin heading {
            fg: $accent;
            modifier: Modifier::BOLD;
        }
    }
}

pub(super) fn use_note_heading_styles() {
    stylesheet! {
        @use note_styles as notes;
        H1 { @include notes.heading; }
    };
}
```

Fill in `src/layouts/root/components/note_heading/component.rs` with these contents.

```rust
use leptatui::prelude::*;

use super::style::use_note_heading_styles;

#[component]
pub(in crate::layouts::root) fn NoteHeading(#[prop(into)] text: String) -> impl IntoView {
    use_note_heading_styles();
    view! { <H1>{text}</H1> }
}
```

Now replace the contents of
`src/layouts/root/components/note_heading/mod.rs` with the module
declarations and re-export below. The layout can keep importing the
heading from the same module.

```rust
mod component;
mod style;

pub(in crate::layouts::root) use component::NoteHeading;
```

Run `cargo run`. Tab to the theme button and press Enter twice. The colors
should switch and then switch back while your draft stays in place.

Nested rules express descendant selectors. `&:focus`, `&:active`, `&:insert`,
`&:visual`, and `&:visited` combine supported states with the containing
selector. Beneath a class, `&__element` and `&--modifier` concatenate class
names, useful for component naming conventions. Specificity and source order
choose between rules; inline styles override normal stylesheet rules, while
stylesheet `!important` overrides normal inline declarations. Inheritable
values flow through the tree; do not assume every layout property inherits.

This is the same co-location pattern introduced for the root layout in chapter 6.
A style helper registers rules for its calling component's subtree; keep it
synchronous and terminate `stylesheet!` with a semicolon when returning `()`.
The [stylesheet reference](../reference/stylesheets.md) explains the cascade.

Run `stylesheet_imports`, `style_cascade_showcase`, or
`default_styles_showcase` with `cargo run --example NAME`.

## Complete files for this chapter

Create or replace only the files listed below; retain all other files from
the previous chapter. Keep the Cargo manifest from [chapter 1](01-getting-started.md).

Delete these superseded files: `src/layouts/root/components/note_heading.rs`.

The project now contains:

```text
└── src/
    ├── app.rs
    ├── draft.rs
    ├── layouts/
    │   ├── mod.rs
    │   └── root/
    │       ├── component.rs
    │       ├── components/
    │       │   ├── file_toolbar.rs
    │       │   ├── mod.rs
    │       │   └── note_heading/
    │       │       ├── component.rs
    │       │       ├── mod.rs
    │       │       └── style.rs
    │       ├── mod.rs
    │       └── style.rs
    ├── main.rs
    └── pages/
        ├── edit/
        │   ├── mod.rs
        │   └── page.rs
        ├── mod.rs
        ├── not_found/
        │   ├── mod.rs
        │   └── page.rs
        ├── note_details/
        │   ├── mod.rs
        │   └── page.rs
        ├── preview/
        │   ├── mod.rs
        │   └── page.rs
        └── required_file/
            ├── mod.rs
            └── page.rs
```

Run `cargo run` in your application project. Ctrl+Q quits.

### `src/app.rs`

```rust
use leptatui::prelude::*;

use crate::draft::Draft;
use crate::layouts::FileToolbar;
use crate::layouts::RootLayout;
use crate::pages::{NotFound, NoteDetails, NoteEditor, NotePreview, RequiredFile};

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

    let light = RwSignal::new(false);
    let theme = RwSignal::new(
        ThemeVariables::new()
            .color("text", Color::White)
            .color("surface", Color::Black),
    );
    provide_context(theme.read_only());

    view! {
        <Router initial_path="/edit">
            <RootLayout>
                <Div class="notes-shell__actions">
                    <A href="/edit">"Edit"</A>
                    <A href="/preview">"Preview"</A>
                    <Button on_press=move || {
                        light.update(|value| *value = !*value);
                        let (text, surface) = if light.get_untracked() {
                            (Color::Black, Color::White)
                        } else {
                            (Color::White, Color::Black)
                        };
                        theme.set(ThemeVariables::new().color("text", text).color("surface", surface));
                    }>"Toggle theme"</Button>
                    <A href="/notes/1?line=3">"Note details"</A>
                    <A href="/notes/abc">"Invalid note"</A>
                    <A href="/required-file">"Read note.md"</A>
                </Div>
                <FileToolbar />
                <Routes fallback=NotFound>
                    <Route path="/edit" view=NoteEditor />
                    <Route path="/preview" view=NotePreview />
                    <Route path="/notes/:id" view=NoteDetails />
                    <Route path="/required-file" view=RequiredFile />
                </Routes>
            </RootLayout>
        </Router>
    }
}
```

### `src/layouts/root/components/note_heading/component.rs`

```rust
use leptatui::prelude::*;

use super::style::use_note_heading_styles;

#[component]
pub(in crate::layouts::root) fn NoteHeading(#[prop(into)] text: String) -> impl IntoView {
    use_note_heading_styles();
    view! { <H1>{text}</H1> }
}
```

### `src/layouts/root/components/note_heading/mod.rs`

```rust
mod component;
mod style;

pub(in crate::layouts::root) use component::NoteHeading;
```

### `src/layouts/root/components/note_heading/style.rs`

```rust
use leptatui::prelude::*;

fn note_styles() -> StyleModule {
    stylesheet! {
        $accent: Color::LightCyan;
        @mixin heading {
            fg: $accent;
            modifier: Modifier::BOLD;
        }
    }
}

pub(super) fn use_note_heading_styles() {
    stylesheet! {
        @use note_styles as notes;
        H1 { @include notes.heading; }
    };
}
```

### `src/layouts/root/style.rs`

```rust
use leptatui::prelude::*;

pub(super) fn use_root_layout_styles() {
    stylesheet! {
        A:active { fg: Color::LightGreen; }
        Button:focus { fg: Color::Yellow; }

        .notes-shell {
            fg: theme_color("text");
            bg: theme_color("surface");
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

[Previous: Documents and media](11-documents-and-media.md) · [Documentation index](../README.md) · [Next: Extending Leptatui](13-extending-leptatui.md)
