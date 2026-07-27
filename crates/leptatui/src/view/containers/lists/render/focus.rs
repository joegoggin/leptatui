//! Focused-descendant geometry for semantic lists.

use ratatui::layout::Rect;

use super::{
    layout::{horizontal_inset, list_item_child_indent, list_markers},
    measure::measure_list_item,
};
use crate::{
    component::RenderCtx,
    view::{
        AnyView, ListItemView, StyleMetadata,
        core::{
            measurement::{cells_to_u16, measure_view},
            render::{VerticalSpan, focused_control_span_for_view, resolve_style},
        },
    },
};

/// Returns the focused descendant span inside a semantic list.
///
/// # Arguments
///
/// * `items` — List item views to inspect in source order.
/// * `ordered_start` — First decimal marker, or [`None`] for hyphen markers.
/// * `metadata` — Selector metadata for the list container.
/// * `ctx` — Rendering context containing the list's assigned area.
///
/// # Returns
///
/// An [`Option`] containing the focused descendant's vertical span.
pub(crate) fn focused_control_span_for_list_view(
    items: &[AnyView],
    ordered_start: Option<usize>,
    metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> Option<VerticalSpan> {
    let style = resolve_style(metadata, ctx);
    let (_, marker_width) = list_markers(items.len(), ordered_start);
    let area = ctx.area();

    ctx.with_area_inherited_style_and_selector_ancestor(
        area,
        style.inherited_values(),
        metadata.clone(),
        |ctx| {
            let mut row = 0u32;

            for item in items {
                let item_height = cells_to_u16(measure_list_item(item, marker_width, ctx).height);
                let item_area = Rect {
                    height: item_height,
                    ..area
                };
                if let Some(span) = ctx.with_area(item_area, |ctx| {
                    focused_control_span_for_list_item(item, marker_width, ctx)
                }) {
                    return Some(span.offset_by(row));
                }

                row = row.saturating_add(u32::from(item_height));
            }

            None
        },
    )
}

/// Returns the focused descendant span inside one marked list item.
///
/// # Arguments
///
/// * `item` — List item or fallback block view to inspect.
/// * `marker_width` — Shared marker-column width for the containing list.
/// * `ctx` — Rendering context containing the item area.
///
/// # Returns
///
/// An [`Option`] containing the focused descendant's vertical span.
fn focused_control_span_for_list_item(
    item: &AnyView,
    marker_width: u16,
    ctx: &mut RenderCtx<'_, '_>,
) -> Option<VerticalSpan> {
    if let Some(item) = item.downcast_ref::<ListItemView>() {
        let style = resolve_style(&item.metadata, ctx);
        let area = ctx.area();
        return ctx.with_area_inherited_style_and_selector_ancestor(
            area,
            style.inherited_values(),
            item.metadata.clone(),
            |ctx| focused_control_span_for_list_item_children(&item.children, marker_width, ctx),
        );
    }

    focused_control_span_for_list_item_children(std::slice::from_ref(item), marker_width, ctx)
}

/// Returns the focused descendant span among stacked list-item blocks.
///
/// # Arguments
///
/// * `children` — Document blocks contained by the item.
/// * `marker_width` — Shared marker-column width for the containing list.
/// * `ctx` — Rendering context containing the item area.
///
/// # Returns
///
/// An [`Option`] containing the focused descendant's vertical span.
fn focused_control_span_for_list_item_children(
    children: &[AnyView],
    marker_width: u16,
    ctx: &mut RenderCtx<'_, '_>,
) -> Option<VerticalSpan> {
    let area = ctx.area();
    let mut row = 0u32;

    for child in children {
        let indent = list_item_child_indent(child, marker_width);
        let child_base = horizontal_inset(area, indent);
        let child_height = ctx.with_area(child_base, |ctx| {
            cells_to_u16(measure_view(child.as_view(), ctx).height)
        });
        let child_area = Rect {
            height: child_height,
            ..child_base
        };
        if let Some(span) =
            ctx.with_area(child_area, |ctx| focused_control_span_for_view(child, ctx))
        {
            return Some(span.offset_by(row));
        }

        row = row.saturating_add(u32::from(child_height));
    }

    None
}
