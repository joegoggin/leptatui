# Layout reference

Start with [Styling and layout](../guide/06-styling-and-layout.md). These sections describe the supported layout contract.

## On this page

- [Layout and Typed Styles](#layout-and-typed-styles)
- [Div and Box Geometry](#div-and-box-geometry)
- [Overflow Contract](#overflow-contract)
- [Flexbox Contract](#flexbox-contract)
- [Positioning Contract](#positioning-contract)
- [Grid Contract](#grid-contract)
- [Supported CSS Differences](#supported-css-differences)

## Layout and Typed Styles

Leptatui exposes layout as Rust values rather than parsed CSS strings or
layout-engine types. Apply a `TuiStyle` directly with a view's `style`
attribute or declare the same properties in `stylesheet!`. Inline and
stylesheet declarations both resolve into one computed style before every
root layout pass.

```rust
use leptatui::prelude::*;

let inline = TuiStyle::new()
    .display(Display::Flex)
    .size(LayoutSize::new(
        Dimension::from(Length::percent(100.0)),
        Dimension::from(Length::vh(50.0)),
    ))
    .gap(Axes::all(Length::cells(1.0)))
    .overflow(Axes::new(Overflow::Hidden, Overflow::Auto));

let panel = view! {
    <Div style={inline}>
        <Text>"Typed layout values"</Text>
    </Div>
};
```

The public layout properties and their value types are:

| Properties | Public value types |
| --- | --- |
| `display`, `box_sizing`, `overflow` | `Display`, `BoxSizing`, `Axes<Overflow>` |
| `size`, `min_size`, `max_size`, `aspect_ratio` | `LayoutSize<Dimension>`, `f32` |
| `margin`, `padding`, `gap` | `Edges<LengthAuto>`, `TuiSpacing`, `Axes<Length>` |
| `flex_direction`, `flex_wrap` | `FlexDirection`, `FlexWrap` |
| `flex_basis`, `flex_grow`, `flex_shrink` | `Dimension`, `f32` |
| `align_items`, `align_self`, `align_content` | `AlignItems`, `AlignSelf`, `AlignContent` |
| `justify_content`, `justify_items`, `justify_self` | `JustifyContent`, `JustifyItems`, `JustifySelf` |
| `grid_template_rows`, `grid_template_columns` | `Vec<GridTemplateTrack>` |
| `grid_auto_rows`, `grid_auto_columns`, `grid_auto_flow` | `Vec<GridTrackSize>`, `GridAutoFlow` |
| `grid_row`, `grid_column` | `GridLine` |
| `position`, `inset`, `z_index` | `Position`, `Edges<LengthAuto>`, `ZIndex` |

`Length` supports terminal cells, containing-block percentages, and `vw`,
`vh`, `vmin`, and `vmax` terminal viewport units. `Dimension` adds `Auto`,
`MinContent`, `MaxContent`, and `FitContent`. General box dimensions currently
treat min-content and max-content like auto, and fit-content like its contained
length; grid track sizing supports its intrinsic variants directly. Non-finite
values are sanitized before reaching layout. Negative `Length` and factor
values are clamped to zero, while non-positive aspect ratios fall back to
automatic sizing.
## Div and Box Geometry

`Div` is the generic multi-child layout container. It defaults to
`Display::Block`, so direct children stack vertically in source order.
`Block` is a bordered single-child container that participates in the same
computed layout. Use `Div` for structure and `Block` when the container itself
needs Ratatui border chrome. `Display::Flex` and `Display::Grid` change how a
container places direct children; `Display::None` removes the entire subtree
from measurement, painting, focus, and input.

Every visible styleable view retains a `LayoutGeometry` after the root layout
pass. `border_box` includes content, padding, and borders; `padding_box`
excludes borders; `content_box` excludes borders and padding; `viewport`
subtracts scrollbar gutters; and `clip` records the accumulated ancestor
clip. `BoxSizing::ContentBox` applies authored sizes to the content box, while
`BoxSizing::BorderBox` includes padding and borders in those sizes. Margins
remain outside the retained border box.

Layout is calculated with floating-point values and then retained as terminal
cell rectangles. Siblings and tracks are rounded cumulatively, preserving
contiguous edges and assigning the final remainder to the sequence instead of
rounding every item independently.

Run the block flow, box sizing, and rounding example:

```sh
cargo run --example block_layout
```

## Overflow Contract

Overflow is configured independently for the horizontal and vertical axes.
Without an authored value, layout uses visible horizontal overflow and
automatic vertical overflow.

- `Overflow::Visible` paints outside the content viewport without clipping or
  scrolling.
- `Overflow::Hidden` clips excess content and retains programmatic,
  keyboard, focus, and wheel scrolling.
- `Overflow::Clip` clips without creating a scroll container.
- `Overflow::Scroll` always enables scrolling and reserves a one-cell terminal
  scrollbar gutter.
- `Overflow::Auto` clips, scrolls, and reserves a gutter only when direct child
  geometry exceeds the available axis.

Scroll ranges are derived from direct layout children; visible overflow inside
a child does not enlarge its parent's range. Nested clips compose during
painting and hit testing. Pointer wheel events target the frontmost
overflowing box under the pointer and bubble to an ancestor at a boundary.
Vertical scrolling also supports Up/Down, `j`/`k`, Page Up/Page Down,
Ctrl-U/Ctrl-D, `gg`, and `G`. Focus traversal scrolls controls into view.

Run the nested two-axis overflow example:

```sh
cargo run --example nested_overflow
```

## Flexbox Contract

Set `display: Display::Flex` on a container to lay out its direct children on
one main axis. Flex containers default to `FlexDirection::Row` and
`FlexWrap::NoWrap`. Items default to an automatic intrinsic basis, no growth,
and a shrink factor of `1.0`; gaps default to zero. Container alignment uses
the layout engine defaults: items and lines stretch on the cross axis, while
main-axis content starts at the selected edge.

The public flex contract includes:

- `flex_direction`: row, row-reverse, column, and column-reverse.
- `flex_wrap`: no-wrap, wrap, and wrap-reverse.
- `gap`: independent horizontal and vertical item or line gaps.
- `flex_basis`, `flex_grow`, and `flex_shrink`: preferred item size and
  positive or negative free-space distribution.
- `justify_content`: main-axis packing and space distribution.
- `align_items` and `align_self`: container and per-item cross-axis alignment.
- `align_content`: wrapped-line distribution on the cross axis.

Text, controls, blocks, and nested layout containers participate through their
intrinsic measurements. Media queries can change flex properties when the
terminal viewport changes, causing layout and measurement to run again.
Engine-owned fractional geometry is rounded cumulatively when retained as
terminal rectangles so adjacent children remain contiguous and the final item
ends at the expected parent edge.

Run the responsive navigation, content, and sidebar example:

```sh
cargo run --example responsive_flex
```

## Positioning Contract

The `position` property selects how a box participates in layout and how its
`inset` edges are resolved. `Position::Static` is the default and ignores
insets. `Position::Relative` keeps its normal-flow space while offsetting its
painted box. `Position::Absolute` leaves normal flow and uses the nearest
non-static ancestor, or the layout root, as its containing block.
`Position::Fixed` also leaves normal flow but is positioned and clipped
against the terminal viewport. `Position::Sticky` keeps its flow space and
clamps its painted box to the authored inset inside the nearest scrollport.

Automatic insets preserve the box's static source position on that axis.
Percentage insets resolve against the relevant containing block and are
recomputed after terminal resizes. Positioned descendants retain ancestor
clipping and scrolling unless fixed positioning explicitly moves them into
the viewport layer.

Positioned boxes with `ZIndex::Auto` share their containing stacking context
and paint in source order at the automatic level. `ZIndex::Integer` assigns a
signed stacking level and establishes an atomic local stacking context,
including for an explicit value of zero. Negative positioned layers paint
after their context's background but before normal-flow content, while
positive layers paint afterward. Pointer targeting follows the resulting
paint order.

Run the static, relative, absolute, fixed, sticky, and stacking example:

```sh
cargo run --example positioning_showcase
```

## Grid Contract

Set `display: Display::Grid` on a container to lay out its direct children in
explicit or automatically created rows and columns. Grid templates use
Leptatui-owned track values rather than exposing layout-engine types. Empty
templates create tracks as automatic placement requires, automatic flow
defaults to row-major sparse packing, and row and column gaps default to zero.

The public grid contract includes:

- `grid_template_columns` and `grid_template_rows`: fixed-cell, percentage,
  fractional, automatic, min-content, max-content, `minmax`, and repeated
  explicit tracks.
- `grid_auto_columns` and `grid_auto_rows`: cyclic sizing patterns for implicit
  tracks created outside the explicit template.
- `grid_auto_flow`: sparse or dense row-major and column-major placement.
- `grid_column` and `grid_row`: signed explicit line pairs, automatic edges,
  and forward or backward spans.
- `gap`: independent horizontal and vertical spacing between tracks.
- `justify_content` and `align_content`: positioning and distribution of the
  grid track area inside the container.
- `justify_items`, `align_items`, `justify_self`, and `align_self`: container
  defaults and per-item alignment overrides.

Text and nested containers contribute intrinsic measurements to automatic and
intrinsic tracks. Fixed tracks and gaps reserve space before fractions consume
the remainder, while item min/max sizes constrain the rectangle inside its
assigned grid area. Media-query changes and terminal resizes rebuild the
template and retained geometry. Fractional engine coordinates are rounded
cumulatively into terminal rectangles so adjacent tracks remain contiguous and
the final track reaches the parent edge.

Run the responsive dashboard example:

```sh
cargo run --example responsive_grid
```

## Supported CSS Differences

Leptatui implements a deliberately typed, terminal-specific subset of
web-style layout. Terminal cells are the absolute unit, all edges are physical
rather than writing-mode-relative, and no DOM, CSS parser, cascade origins,
animations, floats, multicolumn layout, or browser compatibility layer are
provided. Styles are authored through Rust builders and `stylesheet!`; the
underlying layout engine remains private.

Padding and borders are integral terminal cells, scrollbars occupy a cell
gutter, text measurement uses terminal display width, and painting is clipped
to the terminal buffer. Fixed boxes use the terminal viewport rather than a
browser visual viewport. Sticky boxes clamp inside the nearest Leptatui
scrollport. Z-index affects positioned boxes only, and pointer targeting
follows the final terminal paint order. The current API is the supported
contract; superseded row/column splitters and height-only layout paths are not
retained as aliases.

[Documentation index](../README.md) · [Feature index](features.md)
