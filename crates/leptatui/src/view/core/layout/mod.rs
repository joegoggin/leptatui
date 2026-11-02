//! Transient computed-layout integration.
//!
//! Full root layout mirrors visible styleable views into a short-lived Taffy
//! tree, delegates leaf sizing to [`View::measure`](crate::View::measure), and
//! stores rounded engine-independent rectangles on view metadata. Confirmed
//! scroll-only frames can reuse those rectangles and proceed directly to paint.
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
/// The helper builds missing or invalidated layout snapshots, reuses complete
/// snapshots for scroll-only frames, clears stale hit areas, skips hidden
/// boxes, adopts geometry owned by the rendered view, and replays fixed
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
    let reuses_root_layout =
        is_layout_root && ctx.layout_reuse_requested() && retained_layout_is_complete(view, ctx);
    if reuses_root_layout {
        ctx.set_layout_phase(crate::component::LayoutPhase::Paint);
    } else if is_layout_root
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

/// Returns whether every visible layout node retains reusable geometry.
///
/// Layout-transparent boundaries do not need geometry of their own. Custom
/// non-transparent views without style metadata rely on frame-local geometry
/// and therefore force the ordinary layout path.
///
/// # Arguments
///
/// * `view` — Current view subtree being validated.
/// * `ctx` — Render context used to traverse component and stylesheet scopes.
///
/// # Returns
///
/// A [`bool`] indicating whether every visible layout node retains geometry.
fn retained_layout_is_complete(view: &dyn View, ctx: &mut RenderCtx<'_, '_>) -> bool {
    if let Some(metadata) = view.style_metadata() {
        if metadata.is_layout_hidden() {
            return true;
        }
        if metadata.layout_geometry().is_none() {
            return false;
        }
    } else if !view.__is_layout_transparent() {
        return false;
    }

    let mut complete = true;
    view.__visit_layout_children(ctx, &mut |child, child_ctx| {
        if complete {
            complete = retained_layout_is_complete(child.as_view(), child_ctx);
        }
    });
    complete
}

/// Logical child indexes from the rendered root to one layout box.
#[derive(Clone, Debug)]
struct LayoutPath(
    /// Ordered logical child indexes.
    Vec<usize>,
);
