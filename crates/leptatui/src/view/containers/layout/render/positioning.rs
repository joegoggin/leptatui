//! Positioned child style resolution and sticky geometry translation.

use ratatui::layout::Rect;

use crate::view::AnyView;
use crate::view::core::{layout::stacking::StackingLevel, render::resolve_style};
use crate::{
    Axes, Edges, Length, LengthAuto, Position, ViewportSize, ZIndex,
    component::{RenderCtx, StickyScrollport},
    view::core::measurement::sanitize_cells,
};

use super::geometry::translate_rect;

/// Returns the authored positioning and stacking category for one child.
///
/// # Arguments
///
/// * `child` — Child view whose resolved positioning and z-index are inspected.
/// * `ctx` — Render context supplying the active style cascade.
///
/// # Returns
///
/// A [`tuple`](prim@tuple) containing positioning, stacking category, inset
/// edges, and whether the box establishes an explicit stacking context.
pub(super) fn child_paint_style(
    child: &AnyView,
    ctx: &mut RenderCtx<'_, '_>,
) -> (Position, StackingLevel, Edges<LengthAuto>, bool) {
    let Some(metadata) = child.style_metadata() else {
        let mut nested_style = None;
        child
            .as_view()
            .__visit_retained_children(ctx, &mut |nested, nested_ctx| {
                nested_style = nested_style.or_else(|| Some(child_paint_style(nested, nested_ctx)));
            });
        return nested_style.unwrap_or((
            Position::Static,
            StackingLevel::NormalFlow,
            Edges::all(LengthAuto::Auto),
            false,
        ));
    };
    let style = resolve_style(metadata, ctx);
    let position = style.position.unwrap_or_default();
    let z_index = style.z_index.unwrap_or_default();
    let stacking_level = StackingLevel::new(position, z_index);
    let establishes_context = position != Position::Static && matches!(z_index, ZIndex::Integer(_));
    (
        position,
        stacking_level,
        style.inset.unwrap_or_default(),
        establishes_context,
    )
}

/// Returns one child's final painted origin after scrolling and sticky constraints.
///
/// # Arguments
///
/// * `area` — Retained unscrolled child border box.
/// * `offsets` — Parent horizontal and vertical scroll offsets.
/// * `position` — Resolved child positioning behavior.
/// * `sticky_scrollport` — Nearest scrollport constraining sticky descendants.
/// * `insets` — Resolved child inset edges.
/// * `viewport` — Terminal viewport used by viewport-relative lengths.
///
/// # Returns
///
/// A tuple containing signed final x and y terminal coordinates.
pub(super) fn positioned_child_origin(
    area: Rect,
    offsets: Axes<u16>,
    position: Position,
    sticky_scrollport: Option<StickyScrollport>,
    insets: Edges<LengthAuto>,
    viewport: ViewportSize,
) -> (i32, i32) {
    let normal_left = i32::from(area.x) - i32::from(offsets.x);
    let normal_top = i32::from(area.y) - i32::from(offsets.y);
    if position == Position::Sticky {
        sticky_scrollport.map_or((normal_left, normal_top), |scrollport| {
            sticky_position(normal_left, normal_top, area, scrollport, insets, viewport)
        })
    } else {
        (normal_left, normal_top)
    }
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
pub(super) fn translated_geometry(
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
