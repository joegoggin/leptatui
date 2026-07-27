//! Focused-descendant discovery and scroll adjustment.

use crate::view::core::render::{VerticalSpan, focused_control_span_for_view, resolve_style};
use crate::view::{AnyView, StyleMetadata};
use crate::{Axes, Position, component::RenderCtx};

use super::{
    establishes_scrollport,
    geometry::{child_geometry, container_content_area},
    paint::{child_paint_style, positioned_child_origin, translated_geometry},
};

/// Focused descendant bounds relative to a container content box.
#[derive(Clone, Copy)]
pub(super) struct FocusBounds {
    /// First occupied content column.
    pub(super) left: u32,
    /// Column after the focused bounds.
    pub(super) right: u32,
    /// First occupied content row.
    pub(super) top: u32,
    /// Row after the focused bounds.
    pub(super) bottom: u32,
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
pub(super) fn focused_control_bounds_for_container(
    children: &[AnyView],
    metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> Option<FocusBounds> {
    let style = resolve_style(metadata, ctx);
    let (content_area, layout_offset) = container_content_area(metadata, ctx);
    let offsets = metadata.scroll_offsets();
    let sticky_scrollport = if establishes_scrollport(
        style
            .overflow
            .unwrap_or_else(|| Axes::new(crate::Overflow::Visible, crate::Overflow::Auto)),
    ) {
        Some(ctx.layout_geometry().viewport.into())
    } else {
        ctx.sticky_scrollport()
    };
    ctx.with_area_inherited_style_and_selector_ancestor(
        content_area,
        style.inherited_values(),
        metadata.clone(),
        |ctx| {
            children
                .iter()
                .filter(|child| child.__has_scroll_to_anchor_request())
                .chain(
                    children
                        .iter()
                        .filter(|child| !child.__has_scroll_to_anchor_request()),
                )
                .find_map(|child| {
                    let (position, _, insets) = child_paint_style(child, ctx);
                    if position == Position::Fixed {
                        return None;
                    }
                    let geometry = child_geometry(
                        child,
                        content_area,
                        ctx.layout_geometry().clip,
                        layout_offset,
                        ctx,
                    );
                    let (left, top) = positioned_child_origin(
                        geometry.border_box,
                        offsets,
                        position,
                        sticky_scrollport,
                        insets,
                        ctx.viewport_size(),
                    );
                    let geometry = translated_geometry(geometry, left, top);
                    let x_offset = content_offset(left, content_area.x, offsets.x);
                    let y_offset = content_offset(top, content_area.y, offsets.y);
                    with_child_geometry(child, geometry, ctx, |ctx| {
                        focused_or_anchor_span_for_view(child, ctx).map(|span| FocusBounds {
                            left: x_offset,
                            right: x_offset.saturating_add(u32::from(geometry.border_box.width)),
                            top: span.top.saturating_add(y_offset),
                            bottom: span.bottom.saturating_add(y_offset),
                        })
                    })
                })
        },
    )
}

/// Converts one final terminal coordinate back into parent content coordinates.
///
/// # Arguments
///
/// * `position` — Signed final terminal coordinate on one axis.
/// * `content_start` — Parent content-box start on the same axis.
/// * `scroll_offset` — Parent scroll offset on the same axis.
///
/// # Returns
///
/// A `u32` content coordinate suitable for retained scroll calculations.
fn content_offset(position: i32, content_start: u16, scroll_offset: u16) -> u32 {
    let offset = position
        .saturating_sub(i32::from(content_start))
        .saturating_add(i32::from(scroll_offset))
        .max(0);
    u32::try_from(offset).unwrap_or(u32::MAX)
}

/// Runs focused-descendant discovery with one child's final painted geometry.
///
/// # Arguments
///
/// * `child` — Child view searched for focused or anchor content.
/// * `geometry` — Final translated geometry assigned to the child.
/// * `ctx` — Parent render context reproducing style ancestry.
/// * `find` — Callback performing focused-descendant discovery.
///
/// # Returns
///
/// An `R` value returned by `find`.
fn with_child_geometry<R>(
    child: &AnyView,
    geometry: crate::LayoutGeometry,
    ctx: &mut RenderCtx<'_, '_>,
    find: impl FnOnce(&mut RenderCtx<'_, '_>) -> R,
) -> R {
    if let Some(metadata) = child.style_metadata() {
        ctx.with_layout_geometry(geometry, metadata, find)
    } else {
        ctx.with_area(geometry.border_box, find)
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
pub(super) fn scroll_horizontal_bounds_into_view(
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

/// Returns the focused-control or scroll-anchor span for one child view.
///
/// # Arguments
///
/// * `view` — Child view searched for the focused control or scroll anchor.
/// * `ctx` — Render context defining the child's retained area.
///
/// # Returns
///
/// An optional [`VerticalSpan`] when the child contains a focused control or
/// requests anchor scrolling.
fn focused_or_anchor_span_for_view(
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

    focused_control_span_for_view(view, ctx)
}
