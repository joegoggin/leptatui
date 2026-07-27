//! Ordered child painting, clipping, and scrollbar rendering.

use ratatui::{
    layout::Rect,
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::view::core::{layout::stacking::StackingLevel, render::resolve_style};
use crate::view::{AnyView, StyleMetadata};
use crate::{
    Axes, Edges, Length, LengthAuto, Position, TuiStyle, ViewportSize,
    app::Result,
    component::{RenderCtx, StickyScrollport},
    view::core::measurement::sanitize_cells,
};

use super::geometry::{child_geometry, local_geometry, scroll_geometry, translate_rect};

/// Geometry and clipping settings used while painting container children.
#[derive(Clone, Copy)]
pub(super) struct ChildPaintOptions {
    /// Parent content box used for retained child positioning.
    pub(super) content_area: Rect,
    /// Accumulated parent clip applied to each direct child.
    pub(super) clip: Rect,
    /// Signed translation from retained to assigned geometry.
    pub(super) layout_offset: (i32, i32),
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
pub(super) fn render_children(
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
            let (position, stacking_level, insets) = ctx
                .with_area_inherited_style_and_selector_ancestor(
                    options.content_area,
                    inherited_style.clone(),
                    parent_metadata.clone(),
                    |child_ctx| child_paint_style(child, child_ctx),
                );
            (stacking_level, source_index, position, insets, child)
        })
        .collect::<Vec<_>>();
    paint_order
        .sort_by_key(|(stacking_level, source_index, _, _, _)| (*stacking_level, *source_index));
    parent_metadata.set_child_paint_order(
        paint_order
            .iter()
            .map(|(_, source_index, _, _, _)| *source_index),
    );

    for (_, _, position, insets, child) in paint_order {
        if child
            .style_metadata()
            .is_some_and(StyleMetadata::is_layout_hidden)
            || position == Position::Fixed
        {
            continue;
        }

        let geometry = child_geometry(
            child,
            options.content_area,
            options.clip,
            options.layout_offset,
            ctx,
        );
        let full_area = geometry.border_box;
        let normal_left = i32::from(full_area.x) - i32::from(offsets.x);
        let normal_top = i32::from(full_area.y) - i32::from(offsets.y);
        let (shifted_left, shifted_top) = if position == Position::Sticky {
            ctx.sticky_scrollport()
                .map_or((normal_left, normal_top), |scrollport| {
                    sticky_position(
                        normal_left,
                        normal_top,
                        full_area,
                        scrollport,
                        insets,
                        ctx.viewport_size(),
                    )
                })
        } else {
            (normal_left, normal_top)
        };
        let shifted_right = shifted_left.saturating_add(i32::from(full_area.width));
        let shifted_bottom = shifted_top.saturating_add(i32::from(full_area.height));
        let visible_top = shifted_top.max(i32::from(options.clip.y));
        let visible_bottom = shifted_bottom.min(i32::from(options.clip.bottom()));
        let visible_left = shifted_left.max(i32::from(options.clip.x));
        let visible_right = shifted_right.min(i32::from(options.clip.right()));
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
            let geometry = if position == Position::Sticky {
                translated_geometry(geometry, shifted_left, shifted_top)
            } else {
                scroll_geometry(geometry, offsets)
            };
            ctx.with_assigned_layout_geometry_and_selector_ancestor(
                geometry,
                child.style_metadata(),
                inherited_style.clone(),
                parent_metadata.clone(),
                |ctx| child.as_view().render(ctx),
            )?;
        } else {
            let geometry = local_geometry(
                geometry,
                Rect::new(source_x, source_y, visible_area.width, visible_area.height),
            );
            ctx.render_view_clipped(
                child,
                geometry,
                ratatui::layout::Position::new(source_x, source_y),
                visible_area,
                inherited_style.clone(),
                parent_metadata.clone(),
            )?;
        }
    }
    Ok(())
}

/// Returns the authored positioning and stacking category for one child.
///
/// # Arguments
///
/// * `child` — Child view whose resolved positioning and z-index are inspected.
/// * `ctx` — Render context supplying the active style cascade.
///
/// # Returns
///
/// A [`tuple`](prim@tuple) containing positioning, stacking category, and
/// inset edges.
fn child_paint_style(
    child: &AnyView,
    ctx: &RenderCtx<'_, '_>,
) -> (Position, StackingLevel, Edges<LengthAuto>) {
    let Some(metadata) = child.style_metadata() else {
        return (
            Position::Static,
            StackingLevel::NormalFlow,
            Edges::all(LengthAuto::Auto),
        );
    };
    let style = resolve_style(metadata, ctx);
    let position = style.position.unwrap_or_default();
    let stacking_level = StackingLevel::new(position, style.z_index.unwrap_or_default());
    (position, stacking_level, style.inset.unwrap_or_default())
}

/// Returns a sticky box's constrained painted origin.
///
/// # Arguments
///
/// * `normal_left` — Scrolled normal-flow x coordinate.
/// * `normal_top` — Scrolled normal-flow y coordinate.
/// * `area` — Unscrolled retained border box supplying sticky dimensions.
/// * `scrollport` — Nearest scrollport in current target coordinates.
/// * `insets` — Authored sticky inset edges.
/// * `viewport` — Terminal viewport used by viewport-relative lengths.
///
/// # Returns
///
/// A tuple containing constrained x and y coordinates.
fn sticky_position(
    normal_left: i32,
    normal_top: i32,
    area: Rect,
    scrollport: StickyScrollport,
    insets: Edges<LengthAuto>,
    viewport: ViewportSize,
) -> (i32, i32) {
    let left = resolve_sticky_inset(insets.left, scrollport.width, viewport);
    let right = resolve_sticky_inset(insets.right, scrollport.width, viewport);
    let top = resolve_sticky_inset(insets.top, scrollport.height, viewport);
    let bottom = resolve_sticky_inset(insets.bottom, scrollport.height, viewport);
    (
        constrain_sticky_axis(
            normal_left,
            area.width,
            scrollport.x,
            scrollport.width,
            left,
            right,
        ),
        constrain_sticky_axis(
            normal_top,
            area.height,
            scrollport.y,
            scrollport.height,
            top,
            bottom,
        ),
    )
}

/// Constrains one sticky coordinate between optional start and end insets.
///
/// The start edge wins when an oversized box makes opposing constraints
/// impossible to satisfy.
///
/// # Arguments
///
/// * `normal` — Scrolled normal-flow coordinate.
/// * `size` — Sticky border-box size on this axis.
/// * `scrollport_start` — Signed scrollport start coordinate.
/// * `scrollport_size` — Scrollport size on this axis.
/// * `start` — Optional start-edge inset.
/// * `end` — Optional end-edge inset.
///
/// # Returns
///
/// An `i32` containing the constrained painted coordinate.
fn constrain_sticky_axis(
    normal: i32,
    size: u16,
    scrollport_start: i32,
    scrollport_size: u16,
    start: Option<u16>,
    end: Option<u16>,
) -> i32 {
    let minimum = start.map(|inset| scrollport_start.saturating_add(i32::from(inset)));
    let maximum = end.map(|inset| {
        scrollport_start
            .saturating_add(i32::from(scrollport_size))
            .saturating_sub(i32::from(inset))
            .saturating_sub(i32::from(size))
    });
    match (minimum, maximum) {
        (Some(minimum), Some(maximum)) if minimum <= maximum => normal.clamp(minimum, maximum),
        (Some(minimum), _) => normal.max(minimum),
        (None, Some(maximum)) => normal.min(maximum),
        (None, None) => normal,
    }
}

/// Resolves one sticky inset into rounded terminal cells.
///
/// # Arguments
///
/// * `inset` — Authored automatic or definite inset.
/// * `percentage_basis` — Scrollport axis size used by percentages.
/// * `viewport` — Terminal viewport used by viewport-relative lengths.
///
/// # Returns
///
/// An optional `u16` inset, or [`None`] for an automatic edge.
fn resolve_sticky_inset(
    inset: LengthAuto,
    percentage_basis: u16,
    viewport: ViewportSize,
) -> Option<u16> {
    let LengthAuto::Length(length) = inset else {
        return None;
    };
    let width = f32::from(viewport.width);
    let height = f32::from(viewport.height);
    let cells = match length {
        Length::Cells(cells) => cells,
        Length::Percent(percent) => f32::from(percentage_basis) * percent / 100.0,
        Length::ViewportWidth(percent) => width * percent / 100.0,
        Length::ViewportHeight(percent) => height * percent / 100.0,
        Length::ViewportMin(percent) => width.min(height) * percent / 100.0,
        Length::ViewportMax(percent) => width.max(height) * percent / 100.0,
    };
    Some(sanitize_cells(cells).round() as u16)
}

/// Translates retained geometry to a final sticky painted origin.
///
/// # Arguments
///
/// * `geometry` — Retained unscrolled child geometry.
/// * `left` — Final painted x coordinate.
/// * `top` — Final painted y coordinate.
///
/// # Returns
///
/// A [`crate::LayoutGeometry`] translated with its accumulated clip intact.
fn translated_geometry(
    geometry: crate::LayoutGeometry,
    left: i32,
    top: i32,
) -> crate::LayoutGeometry {
    let offset = (
        left.saturating_sub(i32::from(geometry.border_box.x)),
        top.saturating_sub(i32::from(geometry.border_box.y)),
    );
    crate::LayoutGeometry {
        border_box: translate_rect(geometry.border_box, offset),
        padding_box: translate_rect(geometry.padding_box, offset),
        content_box: translate_rect(geometry.content_box, offset),
        viewport: translate_rect(geometry.viewport, offset),
        clip: geometry.clip,
    }
}

/// Renders visible horizontal and vertical scrollbars.
///
/// # Arguments
///
/// * `offsets` — Current horizontal and vertical scroll offsets.
/// * `maximum` — Largest permitted offsets on both axes.
/// * `content_area` — Container content box that receives the scrollbar.
/// * `viewport` — Final content viewport excluding scrollbar gutters.
/// * `gutters` — Whether horizontal and vertical gutters are visible.
/// * `ctx` — Render context targeting the container.
pub(super) fn render_scrollbars(
    offsets: Axes<u16>,
    maximum: Axes<u16>,
    content_area: Rect,
    viewport: Rect,
    gutters: Axes<bool>,
    ctx: &mut RenderCtx<'_, '_>,
) {
    if content_area.width == 0 || content_area.height == 0 {
        return;
    }

    if gutters.y {
        let area = Rect {
            height: viewport.height,
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
            width: viewport.width,
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
