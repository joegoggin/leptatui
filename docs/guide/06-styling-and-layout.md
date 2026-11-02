# 6. Styling and layout

Prerequisites: [Chapter 5: Reusable components](05-reusable-components.md).

Give the notes app a readable, responsive layout. Add this stylesheet inside
`NotesApp`, before its view expression:

```rust
stylesheet! {
    .notes-shell => {
        padding: TuiSpacing::uniform(1)
        .actions => {
            display: Display::Flex,
            gap: Axes::all(Length::cells(1.0))
            @media (max-width: 60) {
                flex_direction: FlexDirection::Column
            }
        }
    }
    H1 => { fg: Color::LightCyan }
    Button:focus => { fg: Color::Yellow }
}
```

Use `<Div class="notes-shell">` around the shell content. Apply
`class="actions"` to its group of action buttons or navigation links.
A stylesheet registered during component setup applies to that component's
subtree, including descendants. Built-in defaults remain available underneath
your rules. A helper containing `stylesheet!` must also run during setup.

## Work in terminal cells

One cell is one terminal character position. `TuiSpacing::uniform(1)` adds
one cell of padding on each side. `Length::cells`, `Length::percent`, and
viewport units (`vw`, `vh`, `vmin`, `vmax`) express typed sizes. `Dimension`
also supports automatic sizing.

To make a container fill the available width, give it this inline style:

```rust
let full_width = TuiStyle::new().size(LayoutSize::new(
    Dimension::from(Length::percent(100.0)),
    Dimension::Auto,
));
let panel = view! { <Div style={full_width}><Text>"Full width"</Text></Div> };
```

`Div` defaults to vertical block flow. Flexbox arranges direct children along
an axis; use `FlexDirection::Column` to stack a row when the terminal narrows.
Grid is useful for aligned dashboard rows and columns:

```rust
let grid = TuiStyle::new()
    .display(Display::Grid)
    .grid_template_columns(vec![
        GridTemplateTrack::from(GridTrackSize::from(Length::cells(20.0))),
        GridTemplateTrack::from(GridTrackSize::from(Fraction::new(1.0))),
    ]);
let layout = view! {
    <Div style={grid}><Text>"Notes"</Text><Text>"Selected note"</Text></Div>
};
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
with `cargo run --example NAME`. Resize the notes app below 60 columns and
confirm that its action row stacks vertically.

For min/max sizing, alignment, grid placement, positioning, sticky/fixed boxes,
stacking, retained geometry, and terminal-specific limitations, use the
[layout reference](../reference/layout.md). For specificity, inheritance,
inline overrides, and `!important`, use the
[stylesheet reference](../reference/stylesheets.md).

[Previous: Reusable components](05-reusable-components.md) · [Documentation index](../README.md) · [Next: Pages and navigation](07-pages-and-navigation.md)
