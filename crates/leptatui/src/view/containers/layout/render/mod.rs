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

mod focus;
mod geometry;
mod measure;
mod paint;

pub(crate) use focus::focused_control_span_for_container;
pub(crate) use measure::measure_container;

use crate::view::core::{
    layout::descendant_clip_rect,
    render::{VerticalSpan, resolve_style, scroll_span_into_view},
};
use crate::view::{AnyView, StyleMetadata};
use crate::{
    Axes, Overflow,
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
    if ctx.honors_layout_geometry() {
        let geometry = metadata
            .layout_geometry()
            .expect("computed containers should retain geometry before painting");
        if geometry != ctx.layout_geometry() {
            return ctx.with_layout_geometry(geometry, metadata, |ctx| {
                render_container(children, metadata, ctx)
            });
        }
    }

    let style = resolve_style(metadata, ctx);
    ctx.record_metadata_hit_area(metadata);
    let geometry = ctx.layout_geometry();
    ctx.with_area(geometry.border_box, |ctx| {
        ctx.render_widget(style.to_block());
    });

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

    if let Some(bounds) = focused_control_bounds_for_container(children, metadata, ctx) {
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
    render_children(
        children,
        offsets,
        style.inherited_values(),
        metadata,
        paint_options,
        ctx,
    )?;
    render_scrollbars(offsets, maximum, content_area, viewport, gutters, ctx);
    Ok(())
}
