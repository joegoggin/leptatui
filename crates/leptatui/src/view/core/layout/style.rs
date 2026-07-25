//! Conversion from resolved Leptatui layout styles to Taffy styles.

use taffy::{
    geometry::{Line as TaffyLine, Point as TaffyPoint, Rect as TaffyRect, Size as TaffySize},
    style::{
        AlignContent as TaffyAlignContent, AlignItems as TaffyAlignItems,
        AlignSelf as TaffyAlignSelf, BoxSizing as TaffyBoxSizing, Dimension as TaffyDimension,
        Display as TaffyDisplay, FlexDirection as TaffyFlexDirection, FlexWrap as TaffyFlexWrap,
        GridAutoFlow as TaffyGridAutoFlow, GridPlacement as TaffyGridPlacement,
        JustifyContent as TaffyJustifyContent, LengthPercentage, LengthPercentageAuto,
        Overflow as TaffyOverflow, Position as TaffyPosition, Style as TaffyStyle,
    },
};

use crate::view::core::measurement::sanitize_cells;
use crate::{
    AlignContent, AlignItems, AlignSelf, Borders, BoxSizing, Dimension, Display, FlexDirection,
    FlexWrap, GridAutoFlow, GridLine, GridPlacement, JustifyContent, JustifyItems, JustifySelf,
    LayoutSize, Length, LengthAuto, Overflow, Position, TuiStyle, View, ViewportSize,
    view::{BlockView, ButtonView, CodeBlockView, InputView, TextAreaView},
};

/// Creates the viewport-sized node used only for multiple transparent roots.
///
/// # Arguments
///
/// * `available` — Assigned terminal area constraining the synthetic root.
///
/// # Returns
///
/// A [`TaffyStyle`] containing definite viewport dimensions.
pub(super) fn synthetic_root_style(available: ViewportSize) -> TaffyStyle {
    TaffyStyle {
        display: TaffyDisplay::Block,
        size: TaffySize {
            width: TaffyDimension::length(f32::from(available.width)),
            height: TaffyDimension::length(f32::from(available.height)),
        },
        ..TaffyStyle::default()
    }
}

/// Converts resolved Leptatui values into one Taffy style.
///
/// # Arguments
///
/// * `view` — View supplying widget-specific border defaults.
/// * `style` — Fully resolved Leptatui style.
/// * `viewport` — Terminal viewport used to resolve viewport-relative lengths.
///
/// # Returns
///
/// A [`TaffyStyle`] containing equivalent engine-owned layout values.
pub(super) fn to_taffy_style(
    view: &dyn View,
    style: TuiStyle,
    viewport: ViewportSize,
) -> TaffyStyle {
    let display = style.display.unwrap_or(Display::Block);
    let flex_direction = style.flex_direction.unwrap_or_default();
    let borders = style.borders.unwrap_or_else(|| default_borders(view));
    let padding = style.padding.unwrap_or_default();
    let overflow = style
        .overflow
        .unwrap_or_else(|| crate::Axes::new(Overflow::Visible, Overflow::Auto));
    let gap = style
        .gap
        .unwrap_or_else(|| crate::Axes::all(Length::Cells(0.0)));

    TaffyStyle {
        display: map_display(display),
        box_sizing: map_box_sizing(style.box_sizing.unwrap_or_default()),
        overflow: TaffyPoint {
            x: map_overflow(overflow.x),
            y: map_overflow(overflow.y),
        },
        scrollbar_width: if overflow.x == Overflow::Scroll || overflow.y == Overflow::Scroll {
            1.0
        } else {
            0.0
        },
        position: map_position(style.position.unwrap_or_default()),
        inset: map_auto_edges(style.inset.unwrap_or_default(), viewport),
        size: map_dimensions(style.size.unwrap_or_default(), viewport),
        min_size: map_dimensions(style.min_size.unwrap_or_default(), viewport),
        max_size: map_dimensions(style.max_size.unwrap_or_default(), viewport),
        aspect_ratio: style.aspect_ratio.and_then(sanitize_aspect_ratio),
        margin: map_auto_edges(
            style
                .margin
                .unwrap_or_else(|| crate::Edges::all(LengthAuto::Length(Length::Cells(0.0)))),
            viewport,
        ),
        padding: TaffyRect {
            left: LengthPercentage::length(f32::from(padding.left)),
            right: LengthPercentage::length(f32::from(padding.right)),
            top: LengthPercentage::length(f32::from(padding.top)),
            bottom: LengthPercentage::length(f32::from(padding.bottom)),
        },
        border: border_edges(borders),
        gap: TaffySize {
            width: map_length(gap.x, viewport),
            height: map_length(gap.y, viewport),
        },
        flex_direction: map_flex_direction(flex_direction),
        flex_wrap: map_flex_wrap(style.flex_wrap.unwrap_or_default()),
        flex_basis: map_dimension(style.flex_basis.unwrap_or_default(), viewport),
        flex_grow: sanitize_factor(style.flex_grow.unwrap_or(0.0)),
        flex_shrink: sanitize_factor(style.flex_shrink.unwrap_or(1.0)),
        align_items: style.align_items.map(map_align_items),
        align_self: style.align_self.and_then(map_align_self),
        align_content: style.align_content.map(map_align_content),
        justify_items: style.justify_items.map(map_justify_items),
        justify_self: style.justify_self.and_then(map_justify_self),
        justify_content: style.justify_content.map(map_justify_content),
        grid_auto_flow: map_grid_auto_flow(style.grid_auto_flow.unwrap_or_default()),
        grid_row: map_grid_line(style.grid_row.unwrap_or_default()),
        grid_column: map_grid_line(style.grid_column.unwrap_or_default()),
        ..TaffyStyle::default()
    }
}

/// Converts a Leptatui display value into Taffy's equivalent.
///
/// # Arguments
///
/// * `value` — Public display value to convert.
///
/// # Returns
///
/// A [`TaffyDisplay`] with matching box-generation behavior.
fn map_display(value: Display) -> TaffyDisplay {
    match value {
        Display::Block => TaffyDisplay::Block,
        Display::Flex => TaffyDisplay::Flex,
        Display::Grid => TaffyDisplay::Grid,
        Display::None => TaffyDisplay::None,
    }
}

/// Converts a Leptatui box-sizing value into Taffy's equivalent.
///
/// # Arguments
///
/// * `value` — Public box-sizing value to convert.
///
/// # Returns
///
/// A [`TaffyBoxSizing`] with matching authored-size semantics.
fn map_box_sizing(value: BoxSizing) -> TaffyBoxSizing {
    match value {
        BoxSizing::ContentBox => TaffyBoxSizing::ContentBox,
        BoxSizing::BorderBox => TaffyBoxSizing::BorderBox,
    }
}

/// Converts one overflow axis into Taffy's layout-affecting equivalent.
///
/// # Arguments
///
/// * `value` — Public overflow behavior to convert.
///
/// # Returns
///
/// A [`TaffyOverflow`] containing the currently supported layout behavior.
fn map_overflow(value: Overflow) -> TaffyOverflow {
    match value {
        Overflow::Visible => TaffyOverflow::Visible,
        Overflow::Hidden | Overflow::Auto => TaffyOverflow::Hidden,
        Overflow::Clip => TaffyOverflow::Clip,
        Overflow::Scroll => TaffyOverflow::Scroll,
    }
}

/// Converts positioning into the subset currently represented by Taffy.
///
/// # Arguments
///
/// * `value` — Public positioning mode to convert.
///
/// # Returns
///
/// A [`TaffyPosition`] containing relative or absolute layout behavior.
fn map_position(value: Position) -> TaffyPosition {
    match value {
        Position::Absolute | Position::Fixed => TaffyPosition::Absolute,
        Position::Static | Position::Relative | Position::Sticky => TaffyPosition::Relative,
    }
}

/// Converts width and height dimensions for the current viewport.
///
/// # Arguments
///
/// * `value` — Authored width and height dimensions.
/// * `viewport` — Terminal viewport used for relative units.
///
/// # Returns
///
/// A [`TaffySize`] containing converted dimensions.
fn map_dimensions(
    value: LayoutSize<Dimension>,
    viewport: ViewportSize,
) -> TaffySize<TaffyDimension> {
    TaffySize {
        width: map_dimension(value.width, viewport),
        height: map_dimension(value.height, viewport),
    }
}

/// Converts one authored dimension for the current viewport.
///
/// # Arguments
///
/// * `value` — Authored dimension to convert.
/// * `viewport` — Terminal viewport used for relative units.
///
/// # Returns
///
/// A [`TaffyDimension`] containing the supported size behavior.
fn map_dimension(value: Dimension, viewport: ViewportSize) -> TaffyDimension {
    match value {
        Dimension::Auto | Dimension::MinContent | Dimension::MaxContent => TaffyDimension::auto(),
        Dimension::Length(length) | Dimension::FitContent(length) => match length {
            Length::Percent(percent) => TaffyDimension::percent(sanitize_percent(percent)),
            length => TaffyDimension::length(resolve_viewport_length(length, viewport)),
        },
    }
}

/// Converts one definite length for the current viewport.
///
/// # Arguments
///
/// * `value` — Definite length to convert.
/// * `viewport` — Terminal viewport used for relative units.
///
/// # Returns
///
/// A [`LengthPercentage`] containing cells or a containing-block ratio.
fn map_length(value: Length, viewport: ViewportSize) -> LengthPercentage {
    match value {
        Length::Percent(percent) => LengthPercentage::percent(sanitize_percent(percent)),
        value => LengthPercentage::length(resolve_viewport_length(value, viewport)),
    }
}

/// Converts one automatic or definite length for the current viewport.
///
/// # Arguments
///
/// * `value` — Automatic or definite length to convert.
/// * `viewport` — Terminal viewport used for relative units.
///
/// # Returns
///
/// A [`LengthPercentageAuto`] containing the converted value.
fn map_auto_length(value: LengthAuto, viewport: ViewportSize) -> LengthPercentageAuto {
    match value {
        LengthAuto::Auto => LengthPercentageAuto::auto(),
        LengthAuto::Length(Length::Percent(percent)) => {
            LengthPercentageAuto::percent(sanitize_percent(percent))
        }
        LengthAuto::Length(length) => {
            LengthPercentageAuto::length(resolve_viewport_length(length, viewport))
        }
    }
}

/// Converts four automatic or definite physical edges.
///
/// # Arguments
///
/// * `value` — Public physical edges to convert.
/// * `viewport` — Terminal viewport used for relative units.
///
/// # Returns
///
/// A [`TaffyRect`] containing converted inset or margin edges.
fn map_auto_edges(
    value: crate::Edges<LengthAuto>,
    viewport: ViewportSize,
) -> TaffyRect<LengthPercentageAuto> {
    TaffyRect {
        left: map_auto_length(value.left, viewport),
        right: map_auto_length(value.right, viewport),
        top: map_auto_length(value.top, viewport),
        bottom: map_auto_length(value.bottom, viewport),
    }
}

/// Resolves cell and viewport-relative lengths into finite terminal cells.
///
/// # Arguments
///
/// * `value` — Public length to resolve.
/// * `viewport` — Terminal viewport supplying relative axis sizes.
///
/// # Returns
///
/// A finite `f32` terminal-cell length.
fn resolve_viewport_length(value: Length, viewport: ViewportSize) -> f32 {
    let width = f32::from(viewport.width);
    let height = f32::from(viewport.height);
    let resolved = match value {
        Length::Cells(cells) => cells,
        Length::Percent(percent) => percent,
        Length::ViewportWidth(percent) => width * percent / 100.0,
        Length::ViewportHeight(percent) => height * percent / 100.0,
        Length::ViewportMin(percent) => width.min(height) * percent / 100.0,
        Length::ViewportMax(percent) => width.max(height) * percent / 100.0,
    };
    sanitize_cells(resolved)
}

/// Converts a public `0..100` percentage into Taffy's finite ratio.
///
/// # Arguments
///
/// * `value` — Percentage where `100.0` represents the full containing axis.
///
/// # Returns
///
/// A finite non-negative `f32` ratio.
fn sanitize_percent(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0) / 100.0
    } else {
        0.0
    }
}

/// Returns a finite non-negative flex factor.
///
/// # Arguments
///
/// * `value` — Authored growth or shrink factor.
///
/// # Returns
///
/// A finite non-negative `f32` factor.
fn sanitize_factor(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

/// Returns a finite positive width-to-height ratio.
///
/// # Arguments
///
/// * `value` — Authored preferred aspect ratio.
///
/// # Returns
///
/// A finite positive ratio, or [`None`] when the value cannot constrain
/// layout safely.
fn sanitize_aspect_ratio(value: f32) -> Option<f32> {
    (value.is_finite() && value > 0.0).then_some(value)
}

/// Converts a flex main-axis direction.
///
/// # Arguments
///
/// * `value` — Public flex direction to convert.
///
/// # Returns
///
/// A [`TaffyFlexDirection`] with matching axis and ordering.
fn map_flex_direction(value: FlexDirection) -> TaffyFlexDirection {
    match value {
        FlexDirection::Row => TaffyFlexDirection::Row,
        FlexDirection::RowReverse => TaffyFlexDirection::RowReverse,
        FlexDirection::Column => TaffyFlexDirection::Column,
        FlexDirection::ColumnReverse => TaffyFlexDirection::ColumnReverse,
    }
}

/// Converts a flex wrapping mode.
///
/// # Arguments
///
/// * `value` — Public flex wrapping mode to convert.
///
/// # Returns
///
/// A [`TaffyFlexWrap`] with matching line behavior.
fn map_flex_wrap(value: FlexWrap) -> TaffyFlexWrap {
    match value {
        FlexWrap::NoWrap => TaffyFlexWrap::NoWrap,
        FlexWrap::Wrap => TaffyFlexWrap::Wrap,
        FlexWrap::WrapReverse => TaffyFlexWrap::WrapReverse,
    }
}

/// Converts container cross-axis item alignment.
///
/// # Arguments
///
/// * `value` — Public item alignment to convert.
///
/// # Returns
///
/// A [`TaffyAlignItems`] with matching alignment behavior.
fn map_align_items(value: AlignItems) -> TaffyAlignItems {
    match value {
        AlignItems::Start => TaffyAlignItems::START,
        AlignItems::End => TaffyAlignItems::END,
        AlignItems::FlexStart => TaffyAlignItems::FLEX_START,
        AlignItems::FlexEnd => TaffyAlignItems::FLEX_END,
        AlignItems::Center => TaffyAlignItems::CENTER,
        AlignItems::Baseline => TaffyAlignItems::BASELINE,
        AlignItems::Stretch => TaffyAlignItems::STRETCH,
    }
}

/// Converts item cross-axis alignment, preserving automatic inheritance.
///
/// # Arguments
///
/// * `value` — Public self-alignment to convert.
///
/// # Returns
///
/// An optional [`TaffyAlignSelf`] omitted for automatic inheritance.
fn map_align_self(value: AlignSelf) -> Option<TaffyAlignSelf> {
    match value {
        AlignSelf::Auto => None,
        AlignSelf::Start => Some(TaffyAlignSelf::START),
        AlignSelf::End => Some(TaffyAlignSelf::END),
        AlignSelf::FlexStart => Some(TaffyAlignSelf::FLEX_START),
        AlignSelf::FlexEnd => Some(TaffyAlignSelf::FLEX_END),
        AlignSelf::Center => Some(TaffyAlignSelf::CENTER),
        AlignSelf::Baseline => Some(TaffyAlignSelf::BASELINE),
        AlignSelf::Stretch => Some(TaffyAlignSelf::STRETCH),
    }
}

/// Converts grid inline-axis item alignment.
///
/// # Arguments
///
/// * `value` — Public grid item alignment to convert.
///
/// # Returns
///
/// A [`TaffyAlignItems`] with matching inline-axis behavior.
fn map_justify_items(value: JustifyItems) -> TaffyAlignItems {
    match value {
        JustifyItems::Start => TaffyAlignItems::START,
        JustifyItems::End => TaffyAlignItems::END,
        JustifyItems::Center => TaffyAlignItems::CENTER,
        JustifyItems::Baseline => TaffyAlignItems::BASELINE,
        JustifyItems::Stretch => TaffyAlignItems::STRETCH,
    }
}

/// Converts grid inline-axis self alignment, preserving automatic inheritance.
///
/// # Arguments
///
/// * `value` — Public grid self-alignment to convert.
///
/// # Returns
///
/// An optional [`TaffyAlignSelf`] omitted for automatic inheritance.
fn map_justify_self(value: JustifySelf) -> Option<TaffyAlignSelf> {
    match value {
        JustifySelf::Auto => None,
        JustifySelf::Start => Some(TaffyAlignSelf::START),
        JustifySelf::End => Some(TaffyAlignSelf::END),
        JustifySelf::Center => Some(TaffyAlignSelf::CENTER),
        JustifySelf::Baseline => Some(TaffyAlignSelf::BASELINE),
        JustifySelf::Stretch => Some(TaffyAlignSelf::STRETCH),
    }
}

/// Converts cross-axis content distribution.
///
/// # Arguments
///
/// * `value` — Public content alignment to convert.
///
/// # Returns
///
/// A [`TaffyAlignContent`] with matching distribution behavior.
fn map_align_content(value: AlignContent) -> TaffyAlignContent {
    match value {
        AlignContent::Start => TaffyAlignContent::START,
        AlignContent::End => TaffyAlignContent::END,
        AlignContent::FlexStart => TaffyAlignContent::FLEX_START,
        AlignContent::FlexEnd => TaffyAlignContent::FLEX_END,
        AlignContent::Center => TaffyAlignContent::CENTER,
        AlignContent::Stretch => TaffyAlignContent::STRETCH,
        AlignContent::SpaceBetween => TaffyAlignContent::SPACE_BETWEEN,
        AlignContent::SpaceAround => TaffyAlignContent::SPACE_AROUND,
        AlignContent::SpaceEvenly => TaffyAlignContent::SPACE_EVENLY,
    }
}

/// Converts main-axis or inline-axis content distribution.
///
/// # Arguments
///
/// * `value` — Public content justification to convert.
///
/// # Returns
///
/// A [`TaffyJustifyContent`] with matching distribution behavior.
fn map_justify_content(value: JustifyContent) -> TaffyJustifyContent {
    match value {
        JustifyContent::Start => TaffyJustifyContent::START,
        JustifyContent::End => TaffyJustifyContent::END,
        JustifyContent::FlexStart => TaffyJustifyContent::FLEX_START,
        JustifyContent::FlexEnd => TaffyJustifyContent::FLEX_END,
        JustifyContent::Center => TaffyJustifyContent::CENTER,
        JustifyContent::Stretch => TaffyJustifyContent::STRETCH,
        JustifyContent::SpaceBetween => TaffyJustifyContent::SPACE_BETWEEN,
        JustifyContent::SpaceAround => TaffyJustifyContent::SPACE_AROUND,
        JustifyContent::SpaceEvenly => TaffyJustifyContent::SPACE_EVENLY,
    }
}

/// Converts automatic grid placement flow.
///
/// # Arguments
///
/// * `value` — Public automatic-flow mode to convert.
///
/// # Returns
///
/// A [`TaffyGridAutoFlow`] with matching axis and density.
fn map_grid_auto_flow(value: GridAutoFlow) -> TaffyGridAutoFlow {
    match value {
        GridAutoFlow::Row => TaffyGridAutoFlow::Row,
        GridAutoFlow::Column => TaffyGridAutoFlow::Column,
        GridAutoFlow::RowDense => TaffyGridAutoFlow::RowDense,
        GridAutoFlow::ColumnDense => TaffyGridAutoFlow::ColumnDense,
    }
}

/// Converts both placements for one grid axis.
///
/// # Arguments
///
/// * `value` — Public start and end placements to convert.
///
/// # Returns
///
/// A [`TaffyLine`] containing both converted placements.
fn map_grid_line(value: GridLine) -> TaffyLine<TaffyGridPlacement> {
    TaffyLine {
        start: map_grid_placement(value.start),
        end: map_grid_placement(value.end),
    }
}

/// Converts one grid edge placement.
///
/// # Arguments
///
/// * `value` — Public grid placement to convert.
///
/// # Returns
///
/// A [`TaffyGridPlacement`] containing automatic, line, or span placement.
fn map_grid_placement(value: GridPlacement) -> TaffyGridPlacement {
    match value {
        GridPlacement::Auto => TaffyGridPlacement::Auto,
        GridPlacement::Line(line) => TaffyGridPlacement::Line(line.into()),
        GridPlacement::Span(span) => TaffyGridPlacement::Span(span),
    }
}

/// Returns the widget borders used when no authored value overrides them.
///
/// # Arguments
///
/// * `view` — View whose built-in border behavior is inspected.
///
/// # Returns
///
/// A [`Borders`] bitset containing the widget defaults.
pub(super) fn default_borders(view: &dyn View) -> Borders {
    if view.as_any().is::<BlockView>()
        || view.as_any().is::<ButtonView>()
        || view.as_any().is::<CodeBlockView>()
        || view.as_any().is::<InputView>()
        || view.as_any().is::<TextAreaView>()
    {
        Borders::ALL
    } else {
        Borders::NONE
    }
}

/// Converts enabled terminal border sides into one-cell Taffy edges.
///
/// # Arguments
///
/// * `borders` — Enabled terminal border sides.
///
/// # Returns
///
/// A [`TaffyRect`] containing zero- or one-cell physical edges.
fn border_edges(borders: Borders) -> TaffyRect<LengthPercentage> {
    TaffyRect {
        left: LengthPercentage::length(f32::from(borders.contains(Borders::LEFT))),
        right: LengthPercentage::length(f32::from(borders.contains(Borders::RIGHT))),
        top: LengthPercentage::length(f32::from(borders.contains(Borders::TOP))),
        bottom: LengthPercentage::length(f32::from(borders.contains(Borders::BOTTOM))),
    }
}
