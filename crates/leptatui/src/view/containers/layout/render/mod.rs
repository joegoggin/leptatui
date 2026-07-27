//! Computed container rendering coordination.
//!
//! This module combines retained geometry, focus-driven scrolling, intrinsic
//! measurement, and child painting behind the container view's rendering API.
//!
//! # Modules
//!
//! - [`focus`] — Focused-descendant discovery and scroll adjustment.
//! - [`geometry`] — Retained child geometry resolution and translation.
//! - [`measure`] — Intrinsic container measurement.
//! - [`paint`] — Ordered child painting, clipping, and scrollbars.
//! - [`positioning`] — Positioned style resolution and sticky translation.

mod focus;
mod geometry;
mod measure;
mod paint;
mod positioning;

pub(crate) use focus::focused_control_span_for_container;
pub(crate) use measure::measure_container;

use crate::view::core::{
    layout::descendant_clip_rect,
    render::{VerticalSpan, resolve_style, scroll_span_into_view},
};
use crate::view::{AnyView, StyleMetadata};
use crate::{
    Axes, Borders, Overflow,
    app::Result,
    component::RenderCtx,
    view::containers::layout::render::{
        focus::{focused_control_bounds_for_container, scroll_horizontal_bounds_into_view},
        geometry::container_content_area,
        paint::{ChildPaintOptions, render_children, render_scrollbars},
    },
};

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
    render_container_with_default_borders(children, metadata, Borders::NONE, ctx)
}

/// Renders a generic container with fallback borders and computed child geometry.
///
/// # Arguments
///
/// * `children` — Child views rendered in source order.
/// * `metadata` — Container selector and runtime metadata.
/// * `default_borders` — Border sides used when no authored value overrides them.
/// * `ctx` — Render context targeting the container border box.
///
/// # Returns
///
/// An empty [`Result`] on success.
///
/// # Errors
///
/// Returns [`crate::Error::Io`] if child rendering performs terminal I/O that fails.
pub(crate) fn render_container_with_default_borders(
    children: &[AnyView],
    metadata: &StyleMetadata,
    default_borders: Borders,
    ctx: &mut RenderCtx<'_, '_>,
) -> Result<()> {
    if ctx.honors_layout_geometry() {
        let geometry = metadata
            .layout_geometry()
            .expect("computed containers should retain geometry before painting");
        if geometry != ctx.layout_geometry() {
            return ctx.with_layout_geometry(geometry, metadata, |ctx| {
                render_container_with_default_borders(children, metadata, default_borders, ctx)
            });
        }
    }

    let traversing_stacking_path = ctx.is_stacking_path_traversal();
    let style = resolve_style(metadata, ctx);
    let geometry = ctx.layout_geometry();
    if !traversing_stacking_path {
        ctx.record_metadata_hit_area(metadata);
        ctx.with_area(geometry.border_box, |ctx| {
            ctx.render_widget(style.to_block_with_default_borders(default_borders));
        });
    }

    let (content_area, layout_offset) = container_content_area(metadata, ctx);
    let overflow = style
        .overflow
        .unwrap_or_else(|| Axes::new(Overflow::Visible, Overflow::Auto));
    let viewport = geometry.viewport;
    let maximum = metadata.max_scroll_offsets();
    let gutters = Axes::new(
        viewport.height < content_area.height,
        viewport.width < content_area.width,
    );
    let paint_options = ChildPaintOptions {
        content_area,
        clip: descendant_clip_rect(geometry.clip, viewport, overflow, maximum),
        layout_offset,
    };

    if !traversing_stacking_path
        && let Some(bounds) = focused_control_bounds_for_container(children, metadata, ctx)
    {
        let scroll_to_anchor = children.iter().any(AnyView::__has_scroll_to_anchor_request);
        if scroll_to_anchor {
            metadata.set_scroll_offset(
                u16::try_from(bounds.top.min(u32::from(maximum.y))).unwrap_or(u16::MAX),
            );
        } else {
            scroll_span_into_view(
                metadata,
                VerticalSpan {
                    top: bounds.top,
                    bottom: bounds.bottom,
                },
                viewport.height,
                maximum.y,
            );
        }
        scroll_horizontal_bounds_into_view(metadata, bounds, viewport.width, maximum.x);
    }

    let offsets = metadata.scroll_offsets();
    let sticky_scrollport = if establishes_scrollport(overflow) {
        Some(viewport.into())
    } else {
        ctx.sticky_scrollport()
    };
    ctx.with_sticky_scrollport(sticky_scrollport, |ctx| {
        render_children(
            children,
            offsets,
            style.inherited_values(),
            metadata,
            paint_options,
            ctx,
        )
    })?;
    if !traversing_stacking_path {
        render_scrollbars(offsets, maximum, content_area, viewport, gutters, ctx);
    }
    Ok(())
}

/// Returns whether authored overflow creates a sticky scrollport.
///
/// # Arguments
///
/// * `overflow` — Resolved horizontal and vertical overflow behavior.
///
/// # Returns
///
/// A [`bool`] indicating whether sticky descendants use this container's
/// viewport.
fn establishes_scrollport(overflow: Axes<Overflow>) -> bool {
    [overflow.x, overflow.y]
        .into_iter()
        .any(|axis| matches!(axis, Overflow::Auto | Overflow::Hidden | Overflow::Scroll))
}
