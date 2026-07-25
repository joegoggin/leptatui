//! Computed container rendering, measurement, scrolling, and focus geometry.

use ratatui::{
    layout::Rect,
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::view::core::render::{VerticalSpan, resolve_style, scroll_span_into_view};
use crate::view::{AnyView, StyleMetadata};
use crate::{
    Axes, LayoutSize, Overflow, Position, TuiStyle, ZIndex, app::Result, component::RenderCtx,
};

/// Geometry and clipping settings used while painting container children.
#[derive(Clone, Copy)]
struct ChildPaintOptions {
    /// Parent content box used for retained child positioning.
    content_area: Rect,
    /// Visible content box after scrollbar gutters.
    viewport: Rect,
    /// Signed translation from retained to assigned geometry.
    layout_offset: (i32, i32),
    /// Whether painting is clipped to the content width.
    clips_horizontal: bool,
    /// Whether painting is clipped to the content height.
    clips_vertical: bool,
}

/// Focused descendant bounds relative to a container content box.
#[derive(Clone, Copy)]
struct FocusBounds {
    /// First occupied content column.
    left: u32,
    /// Column after the focused bounds.
    right: u32,
    /// First occupied content row.
    top: u32,
    /// Row after the focused bounds.
    bottom: u32,
}

/// Computed scroll ranges, viewport, and gutter visibility for one container.
#[derive(Clone, Copy)]
struct OverflowLayout {
    /// Content box after subtracting visible scrollbar gutters.
    viewport: Rect,
    /// Maximum horizontal and vertical scroll offsets.
    maximum: Axes<u16>,
    /// Whether horizontal and vertical scrollbars are visible.
    gutters: Axes<bool>,
}

/// Returns the focused control's vertical span inside computed child geometry.
///
/// # Arguments
///
/// * `children` — Child views searched for the focused control.
/// * `metadata` — Container metadata supplying computed content geometry.
/// * `ctx` — Render context used to reproduce inherited style and selector scopes.
///
/// # Returns
///
/// An optional [`VerticalSpan`] relative to the container content box.
pub(crate) fn focused_control_span_for_container(
    children: &[AnyView],
    metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> Option<VerticalSpan> {
    focused_control_bounds_for_container(children, metadata, ctx).map(|bounds| VerticalSpan {
        top: bounds.top,
        bottom: bounds.bottom,
    })
}

/// Returns the focused control's two-axis bounds inside computed child geometry.
///
/// # Arguments
///
/// * `children` — Child views searched for the focused control.
/// * `metadata` — Container metadata supplying computed content geometry.
/// * `ctx` — Render context used to reproduce inherited style and selector scopes.
///
/// # Returns
///
/// An optional [`FocusBounds`] relative to the container content box.
fn focused_control_bounds_for_container(
    children: &[AnyView],
    metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> Option<FocusBounds> {
    let style = resolve_style(metadata, ctx);
    let (content_area, layout_offset) = container_content_area(metadata, ctx.area());
    ctx.with_area_inherited_style_and_selector_ancestor(
        content_area,
        style.inherited_values(),
        metadata.clone(),
        |ctx| {
            children.iter().find_map(|child| {
                let child_area = child_area(child, content_area, layout_offset, ctx);
                let x_offset = u32::from(child_area.x.saturating_sub(content_area.x));
                let y_offset = u32::from(child_area.y.saturating_sub(content_area.y));
                ctx.with_area(child_area, |ctx| {
                    focused_control_span_for_view(child, ctx).map(|span| FocusBounds {
                        left: x_offset,
                        right: x_offset.saturating_add(u32::from(child_area.width)),
                        top: span.top.saturating_add(y_offset),
                        bottom: span.bottom.saturating_add(y_offset),
                    })
                })
            })
        },
    )
}

/// Renders a generic container and its children from retained layout geometry.
///
/// # Arguments
///
/// * `children` — Child views rendered in source order.
/// * `metadata` — Container selector and runtime metadata.
/// * `ctx` — Render context targeting the container border box.
///
/// # Returns
///
/// An empty [`Result`] on success.
///
/// # Errors
///
/// Returns [`crate::Error::Io`] if child rendering performs terminal I/O that fails.
pub(crate) fn render_container(
    children: &[AnyView],
    metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> Result<()> {
    let style = resolve_style(metadata, ctx);
    ctx.record_metadata_hit_area(metadata);
    ctx.render_widget(style.to_block());

    let (content_area, layout_offset) = container_content_area(metadata, ctx.area());
    let overflow = style
        .overflow
        .unwrap_or_else(|| Axes::new(Overflow::Visible, Overflow::Auto));
    let content_extent = content_extent(children, content_area, layout_offset, ctx);
    let overflow_layout = overflow_layout(content_area, content_extent, overflow);
    metadata.set_content_extent(content_extent);
    metadata.set_max_scroll_offsets(overflow_layout.maximum);

    let clips_horizontal = overflow.x != Overflow::Visible;
    let clips_vertical = overflow.y != Overflow::Visible;
    let paint_options = ChildPaintOptions {
        content_area,
        viewport: overflow_layout.viewport,
        layout_offset,
        clips_horizontal,
        clips_vertical,
    };

    if let Some(bounds) = focused_control_bounds_for_container(children, metadata, ctx) {
        let scroll_to_anchor = children.iter().any(AnyView::__has_scroll_to_anchor_request);
        if scroll_to_anchor {
            metadata.set_scroll_offset(
                u16::try_from(bounds.top.min(u32::from(overflow_layout.maximum.y)))
                    .unwrap_or(u16::MAX),
            );
        } else {
            scroll_span_into_view(
                metadata,
                VerticalSpan {
                    top: bounds.top,
                    bottom: bounds.bottom,
                },
                overflow_layout.viewport.height,
                overflow_layout.maximum.y,
            );
        }
        scroll_horizontal_bounds_into_view(
            metadata,
            bounds,
            overflow_layout.viewport.width,
            overflow_layout.maximum.x,
        );
    }

    let offsets = metadata.scroll_offsets();
    render_children(
        children,
        offsets,
        style.inherited_values(),
        metadata,
        paint_options,
        ctx,
    )?;
    render_scrollbars(
        offsets,
        overflow_layout.maximum,
        content_area,
        overflow_layout.gutters,
        ctx,
    );
    Ok(())
}

/// Returns the rounded content extent produced by retained child geometry.
///
/// # Arguments
///
/// * `children` — Child views contributing to the scrollable region.
/// * `content_area` — Container content box establishing the extent origin.
/// * `layout_offset` — Signed translation from retained to assigned geometry.
/// * `ctx` — Render context used for layout-transparent descendants.
///
/// # Returns
///
/// A [`LayoutSize`] containing the content width and height in terminal cells.
fn content_extent(
    children: &[AnyView],
    content_area: Rect,
    layout_offset: (i32, i32),
    ctx: &mut RenderCtx<'_, '_>,
) -> LayoutSize<u16> {
    let mut extent = LayoutSize::all(0);
    for child in children {
        let area = child_area(child, content_area, layout_offset, ctx);
        extent.width = extent
            .width
            .max(area.right().saturating_sub(content_area.x));
        extent.height = extent
            .height
            .max(area.bottom().saturating_sub(content_area.y));
    }
    extent
}

/// Resolves per-axis scroll ranges and mutually affecting scrollbar gutters.
///
/// # Arguments
///
/// * `content_area` — Full content box before reserving gutters.
/// * `extent` — Scrollable content width and height.
/// * `overflow` — Authored horizontal and vertical overflow behavior.
///
/// # Returns
///
/// An [`OverflowLayout`] containing the final viewport, ranges, and gutters.
fn overflow_layout(
    content_area: Rect,
    extent: LayoutSize<u16>,
    overflow: Axes<Overflow>,
) -> OverflowLayout {
    let mut gutters = Axes::new(
        overflow.x == Overflow::Scroll,
        overflow.y == Overflow::Scroll,
    );

    loop {
        let viewport_width = content_area.width.saturating_sub(u16::from(gutters.y));
        let viewport_height = content_area.height.saturating_sub(u16::from(gutters.x));
        let next = Axes::new(
            overflow.x == Overflow::Scroll
                || (overflow.x == Overflow::Auto && extent.width > viewport_width),
            overflow.y == Overflow::Scroll
                || (overflow.y == Overflow::Auto && extent.height > viewport_height),
        );
        if next == gutters {
            break;
        }
        gutters = next;
    }

    let viewport = Rect {
        width: content_area.width.saturating_sub(u16::from(gutters.y)),
        height: content_area.height.saturating_sub(u16::from(gutters.x)),
        ..content_area
    };
    let maximum = Axes::new(
        if matches!(
            overflow.x,
            Overflow::Hidden | Overflow::Scroll | Overflow::Auto
        ) {
            extent.width.saturating_sub(viewport.width)
        } else {
            0
        },
        if matches!(
            overflow.y,
            Overflow::Hidden | Overflow::Scroll | Overflow::Auto
        ) {
            extent.height.saturating_sub(viewport.height)
        } else {
            0
        },
    );

    OverflowLayout {
        viewport,
        maximum,
        gutters,
    }
}

/// Moves the horizontal offset just enough to reveal focused bounds.
///
/// # Arguments
///
/// * `metadata` — Container metadata owning the retained offsets.
/// * `bounds` — Focused descendant bounds in content coordinates.
/// * `viewport_width` — Visible content width after gutters.
/// * `maximum` — Largest permitted horizontal offset.
fn scroll_horizontal_bounds_into_view(
    metadata: &StyleMetadata,
    bounds: FocusBounds,
    viewport_width: u16,
    maximum: u16,
) {
    if viewport_width == 0 {
        return;
    }
    let viewport_width = u32::from(viewport_width);
    let current = u32::from(metadata.scroll_offsets().x.min(maximum));
    let viewport_right = current.saturating_add(viewport_width);
    let width = bounds.right.saturating_sub(bounds.left);
    let next = if bounds.left < current {
        bounds.left
    } else if bounds.right > viewport_right {
        if width > viewport_width {
            bounds.left
        } else {
            bounds.right.saturating_sub(viewport_width)
        }
    } else {
        current
    }
    .min(u32::from(maximum));
    let mut offsets = metadata.scroll_offsets();
    offsets.x = u16::try_from(next).unwrap_or(u16::MAX);
    metadata.set_scroll_offsets(offsets);
}

/// Returns the legacy minimum height of block-flow children.
///
/// # Arguments
///
/// * `children` — Child views measured in source order.
/// * `metadata` — Container metadata supplying inherited style.
/// * `ctx` — Render context used for child measurement.
///
/// # Returns
///
/// A minimum terminal height covering every child.
pub(crate) fn min_height_for_container(
    children: &[AnyView],
    metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> u16 {
    let style = resolve_style(metadata, ctx);
    let area = ctx.area();
    ctx.with_area_inherited_style_and_selector_ancestor(
        area,
        style.inherited_values(),
        metadata.clone(),
        |ctx| {
            let heights = children.iter().map(|child| child.__min_height(ctx));
            if style.display == Some(crate::Display::Flex)
                && matches!(
                    style.flex_direction.unwrap_or_default(),
                    crate::FlexDirection::Row | crate::FlexDirection::RowReverse
                )
            {
                heights.max().unwrap_or(0)
            } else {
                heights.fold(0, u16::saturating_add)
            }
        },
    )
}

/// Returns the focused span for one child view.
///
/// # Arguments
///
/// * `view` — Child view searched for the focused control or scroll anchor.
/// * `ctx` — Render context defining the child's retained area.
///
/// # Returns
///
/// The focused control's vertical span when the child contains one.
fn focused_control_span_for_view(
    view: &AnyView,
    ctx: &mut RenderCtx<'_, '_>,
) -> Option<VerticalSpan> {
    if view
        .style_metadata()
        .is_some_and(StyleMetadata::scroll_to_anchor_requested)
    {
        return Some(VerticalSpan {
            top: 0,
            bottom: u32::from(ctx.area().height),
        });
    }

    view.__focused_button_span(ctx)
        .map(|(top, bottom)| VerticalSpan { top, bottom })
}

/// Renders visible children into computed areas with two-axis scroll offsets.
///
/// # Arguments
///
/// * `children` — Child views rendered in source order.
/// * `offsets` — Horizontal and vertical offsets applied to child geometry.
/// * `inherited_style` — Cascaded style inherited by each child.
/// * `parent_metadata` — Parent metadata supplying selector ancestry.
/// * `options` — Retained-geometry translation and axis clipping settings.
/// * `ctx` — Render context targeting the container.
///
/// # Returns
///
/// An empty [`Result`] on success.
///
/// # Errors
///
/// Returns [`crate::Error::Io`] if child rendering performs terminal I/O that fails.
fn render_children(
    children: &[AnyView],
    offsets: Axes<u16>,
    inherited_style: TuiStyle,
    parent_metadata: &StyleMetadata,
    options: ChildPaintOptions,
    ctx: &mut RenderCtx<'_, '_>,
) -> Result<()> {
    let mut paint_order = children
        .iter()
        .enumerate()
        .map(|(source_index, child)| {
            let stacking_level = ctx.with_area_inherited_style_and_selector_ancestor(
                options.content_area,
                inherited_style,
                parent_metadata.clone(),
                |child_ctx| child_stacking_level(child, child_ctx),
            );
            (stacking_level, source_index, child)
        })
        .collect::<Vec<_>>();
    paint_order.sort_by_key(|(stacking_level, source_index, _)| (*stacking_level, *source_index));

    for (_, _, child) in paint_order {
        if child
            .style_metadata()
            .is_some_and(StyleMetadata::is_layout_hidden)
        {
            continue;
        }

        let full_area = child_area(child, options.content_area, options.layout_offset, ctx);
        let shifted_left = i32::from(full_area.x) - i32::from(offsets.x);
        let shifted_right = shifted_left.saturating_add(i32::from(full_area.width));
        let shifted_top = i32::from(full_area.y) - i32::from(offsets.y);
        let shifted_bottom = shifted_top.saturating_add(i32::from(full_area.height));
        let visible_top = if options.clips_vertical {
            shifted_top.max(i32::from(options.viewport.y))
        } else {
            shifted_top.max(0)
        };
        let visible_bottom = if options.clips_vertical {
            shifted_bottom.min(i32::from(options.viewport.bottom()))
        } else {
            shifted_bottom.min(i32::from(u16::MAX))
        };
        let visible_left = if options.clips_horizontal {
            shifted_left.max(i32::from(options.viewport.x))
        } else {
            shifted_left.max(0)
        };
        let visible_right = if options.clips_horizontal {
            shifted_right.min(i32::from(options.viewport.right()))
        } else {
            shifted_right.min(i32::from(u16::MAX))
        };
        if visible_bottom <= visible_top || visible_right <= visible_left {
            continue;
        }
        let source_x = u16::try_from(visible_left.saturating_sub(shifted_left)).unwrap_or(u16::MAX);
        let source_y = u16::try_from(visible_top.saturating_sub(shifted_top)).unwrap_or(u16::MAX);
        let shifted_area = Rect {
            x: u16::try_from(shifted_left.max(0)).unwrap_or(u16::MAX),
            y: u16::try_from(shifted_top.max(0)).unwrap_or(u16::MAX),
            ..full_area
        };
        let visible_area = Rect {
            x: u16::try_from(visible_left).unwrap_or(u16::MAX),
            y: u16::try_from(visible_top).unwrap_or(u16::MAX),
            width: u16::try_from(visible_right.saturating_sub(visible_left)).unwrap_or(u16::MAX),
            height: u16::try_from(visible_bottom.saturating_sub(visible_top)).unwrap_or(u16::MAX),
        };

        if source_x == 0 && source_y == 0 && visible_area == shifted_area {
            ctx.with_assigned_area_inherited_style_and_selector_ancestor(
                shifted_area,
                inherited_style,
                parent_metadata.clone(),
                |ctx| child.as_view().render(ctx),
            )?;
        } else {
            ctx.render_view_clipped(
                child,
                full_area,
                ratatui::layout::Position::new(source_x, source_y),
                visible_area,
                inherited_style,
                parent_metadata.clone(),
            )?;
        }
    }
    Ok(())
}

/// Returns the authored stacking level for one positioned child.
///
/// # Arguments
///
/// * `child` — Child view whose resolved positioning and z-index are inspected.
/// * `ctx` — Render context supplying the active style cascade.
///
/// # Returns
///
/// An `i32` stacking level, with static and automatic children at level zero.
fn child_stacking_level(child: &AnyView, ctx: &RenderCtx<'_, '_>) -> i32 {
    let Some(metadata) = child.style_metadata() else {
        return 0;
    };
    let style = resolve_style(metadata, ctx);
    if style.position.unwrap_or_default() == Position::Static {
        return 0;
    }
    match style.z_index.unwrap_or_default() {
        ZIndex::Auto => 0,
        ZIndex::Integer(level) => level,
    }
}

/// Returns one child's retained border box or the parent content fallback.
///
/// # Arguments
///
/// * `child` — Child view whose retained geometry is queried.
/// * `fallback` — Parent content box used when no child geometry is retained.
/// * `ctx` — Render context used to visit layout-transparent descendants.
///
/// # Returns
///
/// The union of the child's effective retained border boxes, or `fallback`.
fn child_area(
    child: &AnyView,
    fallback: Rect,
    layout_offset: (i32, i32),
    ctx: &mut RenderCtx<'_, '_>,
) -> Rect {
    if let Some(geometry) = ctx.unstyled_layout_geometry(child.as_view()) {
        return translate_rect(geometry.border_box, layout_offset);
    }
    if let Some(area) = child
        .style_metadata()
        .and_then(StyleMetadata::layout_geometry)
        .map(|geometry| geometry.border_box)
    {
        return translate_rect(area, layout_offset);
    }

    let mut area = None;
    child
        .as_view()
        .__visit_layout_children(ctx, &mut |nested, nested_ctx| {
            let nested_area = child_area(nested, fallback, layout_offset, nested_ctx);
            area = Some(area.map_or(nested_area, |current: Rect| current.union(nested_area)));
        });
    area.unwrap_or(fallback)
}

/// Returns translated content geometry and its retained-to-assigned offset.
///
/// # Arguments
///
/// * `metadata` — Container metadata supplying retained geometry.
/// * `assigned_area` — Area currently assigned by the render parent.
///
/// # Returns
///
/// The assigned content box and signed translation applied to descendants.
fn container_content_area(metadata: &StyleMetadata, assigned_area: Rect) -> (Rect, (i32, i32)) {
    let Some(geometry) = metadata.layout_geometry() else {
        return (assigned_area, (0, 0));
    };
    let offset = (
        i32::from(assigned_area.x) - i32::from(geometry.border_box.x),
        i32::from(assigned_area.y) - i32::from(geometry.border_box.y),
    );
    (translate_rect(geometry.content_box, offset), offset)
}

/// Translates a terminal rectangle by signed cell offsets.
///
/// # Arguments
///
/// * `area` — Retained rectangle to translate.
/// * `offset` — Signed horizontal and vertical cell offsets.
///
/// # Returns
///
/// A rectangle whose origin is clamped to terminal coordinate bounds.
fn translate_rect(area: Rect, offset: (i32, i32)) -> Rect {
    Rect {
        x: u16::try_from((i32::from(area.x) + offset.0).clamp(0, i32::from(u16::MAX)))
            .unwrap_or(u16::MAX),
        y: u16::try_from((i32::from(area.y) + offset.1).clamp(0, i32::from(u16::MAX)))
            .unwrap_or(u16::MAX),
        ..area
    }
}

/// Renders visible horizontal and vertical scrollbars.
///
/// # Arguments
///
/// * `offsets` — Current horizontal and vertical scroll offsets.
/// * `maximum` — Largest permitted offsets on both axes.
/// * `content_area` — Container content box that receives the scrollbar.
/// * `gutters` — Whether horizontal and vertical gutters are visible.
/// * `ctx` — Render context targeting the container.
fn render_scrollbars(
    offsets: Axes<u16>,
    maximum: Axes<u16>,
    content_area: Rect,
    gutters: Axes<bool>,
    ctx: &mut RenderCtx<'_, '_>,
) {
    if content_area.width == 0 || content_area.height == 0 {
        return;
    }

    if gutters.y {
        let area = Rect {
            height: content_area.height.saturating_sub(u16::from(gutters.x)),
            ..content_area
        };
        let mut state = ScrollbarState::new(usize::from(maximum.y).saturating_add(1))
            .position(usize::from(offsets.y))
            .viewport_content_length(usize::from(area.height));
        ctx.with_area(area, |ctx| {
            ctx.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .end_symbol(None),
                &mut state,
            );
        });
    }
    if gutters.x {
        let area = Rect {
            width: content_area.width.saturating_sub(u16::from(gutters.y)),
            ..content_area
        };
        let mut state = ScrollbarState::new(usize::from(maximum.x).saturating_add(1))
            .position(usize::from(offsets.x))
            .viewport_content_length(usize::from(area.width));
        ctx.with_area(area, |ctx| {
            ctx.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
                    .begin_symbol(None)
                    .end_symbol(None),
                &mut state,
            );
        });
    }
}
