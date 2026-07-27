//! Two-axis measurement for semantic lists and list items.

use super::layout::{horizontal_inset, list_item_child_indent, list_markers};
use crate::{
    LayoutSize,
    component::RenderCtx,
    view::{
        AnyView, ListItemView, StyleMetadata,
        core::{
            measurement::{measure_view, sanitize_cells},
            render::resolve_style,
        },
    },
};

/// Measures one marked list item.
///
/// # Arguments
///
/// * `item` — List item or fallback block view to measure.
/// * `marker_width` — Shared marker-column width for the containing list.
/// * `ctx` — Rendering context containing the available width.
///
/// # Returns
///
/// A [`LayoutSize`] including a marker-only row for empty items.
pub(super) fn measure_list_item(
    item: &AnyView,
    marker_width: u16,
    ctx: &mut RenderCtx<'_, '_>,
) -> LayoutSize<f32> {
    if let Some(item) = item.downcast_ref::<ListItemView>() {
        let style = resolve_style(&item.metadata, ctx);
        let area = ctx.area();
        return ctx.with_area_inherited_style_and_selector_ancestor(
            area,
            style.inherited_values(),
            item.metadata.clone(),
            |ctx| measure_list_item_children(&item.children, marker_width, ctx),
        );
    }

    measure_list_item_children(std::slice::from_ref(item), marker_width, ctx)
}

/// Measures the content of an ordered or unordered list.
///
/// # Arguments
///
/// * `items` — Item views to measure.
/// * `ordered_start` — First decimal marker, or [`None`] for hyphen markers.
/// * `metadata` — Selector metadata for the list container.
/// * `ctx` — Rendering context containing the available width.
///
/// # Returns
///
/// A [`LayoutSize`] containing the widest item and stacked item height.
pub(crate) fn measure_list_view(
    items: &[AnyView],
    ordered_start: Option<usize>,
    metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> LayoutSize<f32> {
    let style = resolve_style(metadata, ctx);
    let (_, marker_width) = list_markers(items.len(), ordered_start);
    let area = ctx.area();

    ctx.with_area_inherited_style_and_selector_ancestor(
        area,
        style.inherited_values(),
        metadata.clone(),
        |ctx| {
            items
                .iter()
                .map(|item| measure_list_item(item, marker_width, ctx))
                .fold(LayoutSize::all(0.0_f32), |measured, item| {
                    LayoutSize::new(
                        measured.width.max(item.width),
                        sanitize_cells(measured.height + item.height),
                    )
                })
        },
    )
}

/// Measures the stacked blocks inside one marked list item.
///
/// # Arguments
///
/// * `children` — Document blocks contained by the item.
/// * `marker_width` — Shared marker-column width for the containing list.
/// * `ctx` — Rendering context containing the available width.
///
/// # Returns
///
/// A [`LayoutSize`] with at least one row reserved for the item marker.
fn measure_list_item_children(
    children: &[AnyView],
    marker_width: u16,
    ctx: &mut RenderCtx<'_, '_>,
) -> LayoutSize<f32> {
    let area = ctx.area();
    let measured = children
        .iter()
        .map(|child| {
            let indent = list_item_child_indent(child, marker_width);
            let child_area = horizontal_inset(area, indent);
            let measured = ctx.with_area(child_area, |ctx| measure_view(child.as_view(), ctx));
            LayoutSize::new(
                sanitize_cells(measured.width + f32::from(indent)),
                measured.height,
            )
        })
        .fold(LayoutSize::all(0.0_f32), |measured, child| {
            LayoutSize::new(
                measured.width.max(child.width),
                sanitize_cells(measured.height + child.height),
            )
        });
    LayoutSize::new(measured.width, measured.height.max(1.0))
}
