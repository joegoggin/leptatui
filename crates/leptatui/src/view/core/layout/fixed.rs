//! Deferred terminal-viewport painting for fixed-position descendants.

use crate::{AnyView, Position, RenderCtx, View, app::Result};

/// Paints fixed descendants after their normal-flow root has rendered.
///
/// Fixed descendants retain their logical style and component ancestry while
/// painting from viewport-relative geometry. Walking the logical tree here
/// also allows fixed boxes to escape ancestors that were entirely clipped
/// during normal painting.
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
    let mut result = Ok(());
    visit_retained_children_with_style(root, ctx, &mut |child, child_ctx| {
        if result.is_err() {
            return;
        }

        let hidden = child
            .style_metadata()
            .is_some_and(crate::StyleMetadata::is_layout_hidden);
        if hidden {
            return;
        }

        let fixed = child.style_metadata().is_some_and(|metadata| {
            child_ctx
                .resolve_style(metadata)
                .position
                .unwrap_or_default()
                == Position::Fixed
        });
        if fixed {
            let metadata = child
                .style_metadata()
                .expect("fixed views should expose style metadata");
            let geometry = metadata
                .layout_geometry()
                .expect("fixed views should retain viewport-relative geometry");
            child.as_view().__clear_hit_areas();
            result = child_ctx.with_layout_geometry(geometry, metadata, |fixed_ctx| {
                fixed_ctx.record_metadata_hit_area(metadata);
                child.as_view().render(fixed_ctx)
            });
        }
        if result.is_ok() {
            result = render_fixed_descendants(child.as_view(), child_ctx);
        }
    });
    result
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
