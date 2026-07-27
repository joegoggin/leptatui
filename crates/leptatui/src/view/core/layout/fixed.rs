//! Deferred terminal-viewport painting for fixed-position descendants.

use crate::{AnyView, Position, RenderCtx, View, app::Result};

use super::stacking::StackingLevel;

/// Deferred fixed box and its logical path from the rendered root.
struct FixedPaint {
    /// Child indexes used to reproduce retained style and component scopes.
    path: Vec<usize>,
    /// Viewport stacking category resolved for the fixed box.
    stacking_level: StackingLevel,
    /// Depth-first logical order used to break equal-level ties.
    source_order: usize,
}

/// Paints fixed descendants after their normal-flow root has rendered.
///
/// Fixed descendants retain their logical style and component ancestry while
/// painting from viewport-relative geometry. Walking the logical tree here
/// also allows fixed boxes to escape ancestors that were entirely clipped
/// during normal painting. The collected boxes use the same negative,
/// automatic-or-zero, and positive ordering as ordinary positioned siblings.
///
/// # Arguments
///
/// * `root` — Logical root whose fixed descendants should be painted.
/// * `ctx` — Root render context targeting the terminal viewport.
///
/// # Returns
///
/// An empty [`Result`] after every visible fixed descendant renders.
///
/// # Errors
///
/// Returns [`crate::Error::Io`] if fixed descendant rendering performs
/// terminal I/O that fails.
pub(crate) fn render_fixed_descendants(root: &dyn View, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
    let mut paint_order = Vec::new();
    collect_fixed_descendants(root, ctx, &mut Vec::new(), &mut 0, &mut paint_order);
    paint_order.sort_by_key(|paint| (paint.stacking_level, paint.source_order));
    for paint in paint_order {
        render_fixed_path(root, ctx, &paint.path)?;
    }
    Ok(())
}

/// Collects visible fixed descendants and their viewport stacking levels.
///
/// # Arguments
///
/// * `view` — Parent whose retained logical subtree is searched.
/// * `ctx` — Render context carrying the parent's style ancestry.
/// * `path` — Mutable logical path to the current parent.
/// * `source_order` — Monotonic depth-first order for stable ties.
/// * `paint_order` — Output entries receiving discovered fixed boxes.
fn collect_fixed_descendants(
    view: &dyn View,
    ctx: &mut RenderCtx<'_, '_>,
    path: &mut Vec<usize>,
    source_order: &mut usize,
    paint_order: &mut Vec<FixedPaint>,
) {
    let mut child_index = 0;
    visit_retained_children_with_style(view, ctx, &mut |child, child_ctx| {
        let current_index = child_index;
        child_index += 1;
        let hidden = child
            .style_metadata()
            .is_some_and(crate::StyleMetadata::is_layout_hidden);
        if hidden {
            return;
        }

        path.push(current_index);
        if let Some(metadata) = child.style_metadata() {
            let style = child_ctx.resolve_style(metadata);
            let position = style.position.unwrap_or_default();
            if position == Position::Fixed {
                paint_order.push(FixedPaint {
                    path: path.clone(),
                    stacking_level: StackingLevel::new(position, style.z_index.unwrap_or_default()),
                    source_order: *source_order,
                });
                *source_order = source_order.saturating_add(1);
            }
        }
        collect_fixed_descendants(child.as_view(), child_ctx, path, source_order, paint_order);
        path.pop();
    });
}

/// Replays one logical path and paints its fixed endpoint.
///
/// # Arguments
///
/// * `view` — Current logical parent on the retained path.
/// * `ctx` — Render context carrying the current style ancestry.
/// * `path` — Remaining child indexes leading to the fixed box.
///
/// # Returns
///
/// An empty [`Result`] after the endpoint renders.
///
/// # Errors
///
/// Returns [`crate::Error::Io`] if fixed descendant rendering performs
/// terminal I/O that fails.
fn render_fixed_path(view: &dyn View, ctx: &mut RenderCtx<'_, '_>, path: &[usize]) -> Result<()> {
    let Some((&target_index, remaining)) = path.split_first() else {
        return Ok(());
    };
    let mut result = Ok(());
    let mut child_index = 0;
    visit_retained_children_with_style(view, ctx, &mut |child, child_ctx| {
        let current_index = child_index;
        child_index += 1;
        if result.is_err() || current_index != target_index {
            return;
        }

        if remaining.is_empty() {
            result = paint_fixed_view(child, child_ctx);
        } else {
            result = render_fixed_path(child.as_view(), child_ctx, remaining);
        }
    });
    result
}

/// Paints one fixed view from its retained viewport-relative geometry.
///
/// # Arguments
///
/// * `view` — Fixed view to paint.
/// * `ctx` — Render context reproducing its logical style ancestry.
///
/// # Returns
///
/// An empty [`Result`] after the fixed view renders.
///
/// # Errors
///
/// Returns [`crate::Error::Io`] if fixed descendant rendering performs
/// terminal I/O that fails.
fn paint_fixed_view(view: &AnyView, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
    let metadata = view
        .style_metadata()
        .expect("fixed views should expose style metadata");
    let geometry = metadata
        .layout_geometry()
        .expect("fixed views should retain viewport-relative geometry");
    view.as_view().__clear_hit_areas();
    ctx.with_layout_geometry(geometry, metadata, |fixed_ctx| {
        fixed_ctx.record_metadata_hit_area(metadata);
        view.as_view().render(fixed_ctx)
    })
}

/// Visits retained logical children with their inherited style ancestry.
///
/// # Arguments
///
/// * `view` — Parent whose retained logical children are visited.
/// * `ctx` — Render context active at the parent.
/// * `visitor` — Callback invoked for each child and its scoped context.
fn visit_retained_children_with_style(
    view: &dyn View,
    ctx: &mut RenderCtx<'_, '_>,
    visitor: &mut dyn FnMut(&AnyView, &mut RenderCtx<'_, '_>),
) {
    let Some(metadata) = view.style_metadata() else {
        view.__visit_retained_children(ctx, visitor);
        return;
    };
    let style = ctx.resolve_style(metadata);
    ctx.with_area_inherited_style_and_selector_ancestor(
        ctx.area(),
        style.inherited_values(),
        metadata.clone(),
        |child_ctx| view.__visit_retained_children(child_ctx, visitor),
    );
}
