//! Retained child geometry resolution and translation.

use ratatui::layout::Rect;

use crate::view::{AnyView, StyleMetadata};
use crate::{Axes, LayoutGeometry, component::RenderCtx};

/// Returns one child's translated retained geometry or a parent-area fallback.
///
/// # Arguments
///
/// * `child` — Child view whose retained geometry is queried.
/// * `fallback` — Parent content box used when no child geometry is retained.
/// * `clip` — Parent clip applied to the direct child.
/// * `layout_offset` — Translation from retained to assigned coordinates.
/// * `ctx` — Render context used to visit layout-transparent descendants.
///
/// # Returns
///
/// A [`LayoutGeometry`] for the child in assigned target coordinates.
pub(super) fn child_geometry(
    child: &AnyView,
    fallback: Rect,
    clip: Rect,
    layout_offset: (i32, i32),
    ctx: &mut RenderCtx<'_, '_>,
) -> LayoutGeometry {
    if let Some(geometry) = ctx.unstyled_layout_geometry(child.as_view()) {
        return translated_child_geometry(geometry, clip, layout_offset);
    }
    if let Some(geometry) = child
        .style_metadata()
        .and_then(StyleMetadata::layout_geometry)
    {
        return translated_child_geometry(geometry, clip, layout_offset);
    }

    let mut geometry = None;
    child
        .as_view()
        .__visit_layout_children(ctx, &mut |nested, nested_ctx| {
            let nested_geometry = child_geometry(nested, fallback, clip, layout_offset, nested_ctx);
            geometry =
                Some(
                    geometry.map_or(nested_geometry, |current: LayoutGeometry| LayoutGeometry {
                        border_box: current.border_box.union(nested_geometry.border_box),
                        padding_box: current.padding_box.union(nested_geometry.padding_box),
                        content_box: current.content_box.union(nested_geometry.content_box),
                        viewport: current.viewport.union(nested_geometry.viewport),
                        clip,
                    }),
                );
        });
    geometry.unwrap_or(LayoutGeometry {
        border_box: fallback,
        padding_box: fallback,
        content_box: fallback,
        viewport: fallback,
        clip,
    })
}

/// Translates retained child boxes and replaces their ancestor clip.
///
/// # Arguments
///
/// * `geometry` — Retained absolute geometry from the layout snapshot.
/// * `clip` — Current parent clip in assigned coordinates.
/// * `offset` — Signed translation from retained to assigned coordinates.
///
/// # Returns
///
/// A translated [`LayoutGeometry`] ready for direct-child painting.
fn translated_child_geometry(
    geometry: LayoutGeometry,
    clip: Rect,
    offset: (i32, i32),
) -> LayoutGeometry {
    LayoutGeometry {
        border_box: translate_rect(geometry.border_box, offset),
        padding_box: translate_rect(geometry.padding_box, offset),
        content_box: translate_rect(geometry.content_box, offset),
        viewport: translate_rect(geometry.viewport, offset),
        clip,
    }
}

/// Applies scroll offsets to child boxes while keeping the parent clip fixed.
///
/// # Arguments
///
/// * `geometry` — Assigned child geometry before scrolling.
/// * `offsets` — Horizontal and vertical scroll offsets.
///
/// # Returns
///
/// A [`LayoutGeometry`] translated into its visible scrolled position.
pub(super) fn scroll_geometry(geometry: LayoutGeometry, offsets: Axes<u16>) -> LayoutGeometry {
    let offset = (-i32::from(offsets.x), -i32::from(offsets.y));
    LayoutGeometry {
        border_box: translate_rect(geometry.border_box, offset),
        padding_box: translate_rect(geometry.padding_box, offset),
        content_box: translate_rect(geometry.content_box, offset),
        viewport: translate_rect(geometry.viewport, offset),
        clip: geometry.clip,
    }
}

/// Converts assigned child geometry into offscreen-buffer coordinates.
///
/// # Arguments
///
/// * `geometry` — Assigned child geometry before scrolling.
/// * `source` — Child-local rectangle copied from the offscreen buffer.
///
/// # Returns
///
/// A [`LayoutGeometry`] rooted at buffer coordinate zero and clipped to `source`.
pub(super) fn local_geometry(geometry: LayoutGeometry, source: Rect) -> LayoutGeometry {
    let offset = (
        -i32::from(geometry.border_box.x),
        -i32::from(geometry.border_box.y),
    );
    LayoutGeometry {
        border_box: translate_rect(geometry.border_box, offset),
        padding_box: translate_rect(geometry.padding_box, offset),
        content_box: translate_rect(geometry.content_box, offset),
        viewport: translate_rect(geometry.viewport, offset),
        clip: source,
    }
}

/// Returns translated content geometry and its retained-to-assigned offset.
///
/// # Arguments
///
/// * `metadata` — Container metadata supplying retained geometry.
/// * `ctx` — Context carrying the container's assigned geometry.
///
/// # Returns
///
/// The assigned content box and signed translation applied to descendants.
pub(super) fn container_content_area(
    metadata: &StyleMetadata,
    ctx: &RenderCtx<'_, '_>,
) -> (Rect, (i32, i32)) {
    let active = ctx.layout_geometry();
    let Some(geometry) = metadata.layout_geometry() else {
        return (active.content_box, (0, 0));
    };
    let offset = (
        i32::from(active.border_box.x) - i32::from(geometry.border_box.x),
        i32::from(active.border_box.y) - i32::from(geometry.border_box.y),
    );
    (active.content_box, offset)
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
