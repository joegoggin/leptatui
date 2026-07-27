//! Transient computed-layout integration.
//!
//! Each root render mirrors visible styleable views into a short-lived
//! Taffy tree, delegates leaf sizing to [`View::measure`](crate::View::measure),
//! and stores rounded engine-independent rectangles on view metadata.
//!
//! # Modules
//!
//! - [`fixed`] — Deferred viewport-level painting for fixed descendants.
//! - [`geometry`] — Retained terminal geometry conversion and assignment.
//! - [`measure`] — View measurement and logical-path traversal.
//! - [`stacking`] — Shared web-inspired paint-level classification.
//! - [`style`] — Leptatui-to-Taffy style conversion.
//! - [`tree`] — Layout-tree construction, computation, and orchestration.

mod fixed;
mod geometry;
mod measure;
pub(crate) mod stacking;
mod style;
mod tree;

use crate::{RenderCtx, View, app::Result};

pub(crate) use fixed::render_fixed_descendants;
pub(crate) use geometry::descendant_clip_rect;
pub(crate) use tree::prepare_layout;

/// Renders one concrete view through the shared computed-layout lifecycle.
///
/// The helper builds missing layout snapshots, clears stale hit areas, skips
/// hidden boxes, adopts geometry owned by the rendered view, and replays fixed
/// descendants after a root paint. Concrete containers, erased views, and
/// application roots share it so those responsibilities cannot diverge.
///
/// # Arguments
///
/// * `view` — Concrete view serving as the possible layout root.
/// * `ctx` — Render context carrying the current layout phase.
/// * `paint` — Widget-specific paint operation consuming active geometry.
///
/// # Returns
///
/// An empty [`Result`] after the normal and fixed paint passes complete.
///
/// # Errors
///
/// Returns [`crate::Error::Io`] if widget or fixed-descendant rendering fails.
pub(crate) fn render_with_layout(
    view: &dyn View,
    ctx: &mut RenderCtx<'_, '_>,
    paint: impl FnOnce(&mut RenderCtx<'_, '_>) -> Result<()>,
) -> Result<()> {
    if view
        .style_metadata()
        .is_some_and(|metadata| ctx.active_layout_geometry(metadata).is_some())
    {
        return paint(ctx);
    }

    let is_layout_root = ctx.layout_phase() == crate::component::LayoutPhase::Inactive;
    if is_layout_root
        || view
            .style_metadata()
            .is_some_and(|metadata| metadata.layout_geometry().is_none())
    {
        prepare_layout(view, ctx);
    }
    if !ctx.is_stacking_path_traversal() {
        view.__clear_hit_areas();
    }

    let result = if let Some(metadata) = view.style_metadata() {
        if metadata.is_layout_hidden() {
            Ok(())
        } else if ctx.honors_layout_geometry() {
            let geometry = metadata
                .layout_geometry()
                .expect("styled views should retain geometry before painting");
            ctx.with_layout_geometry(geometry, metadata, |ctx| {
                ctx.record_metadata_hit_area(metadata);
                paint(ctx)
            })
        } else {
            ctx.record_metadata_hit_area(metadata);
            paint(ctx)
        }
    } else {
        paint(ctx)
    };
    result?;
    if is_layout_root {
        render_fixed_descendants(view, ctx)?;
    }
    Ok(())
}

/// Logical child indexes from the rendered root to one layout box.
#[derive(Clone, Debug)]
struct LayoutPath(
    /// Ordered logical child indexes.
    Vec<usize>,
);
