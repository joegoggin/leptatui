//! Terminal painting for semantic lists and list items.

use ratatui::{
    layout::Rect,
    widgets::{Block, Paragraph},
};

use super::{
    layout::{horizontal_inset, list_item_child_indent, list_markers},
    measure::measure_list_item,
};
use crate::{
    TuiStyle,
    app::Result,
    component::RenderCtx,
    view::{
        AnyView, ListItemView, StyleMetadata,
        core::{
            measurement::{cells_to_u16, measure_view},
            render::resolve_style,
        },
    },
};

/// Renders a semantic ordered or unordered list.
///
/// # Arguments
///
/// * `items` — Item views to render in source order.
/// * `ordered_start` — First decimal marker, or [`None`] for hyphen markers.
/// * `metadata` — Selector metadata for the list container.
/// * `ctx` — Rendering context for the list area.
///
/// # Returns
///
/// An empty [`Result`] on success.
///
/// # Errors
///
/// Returns [`crate::app::Error::Io`] if child rendering performs terminal I/O
/// that fails.
pub(crate) fn render_list_view(
    items: &[AnyView],
    ordered_start: Option<usize>,
    metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> Result<()> {
    let style = resolve_style(metadata, ctx);
    let geometry = ctx.active_layout_geometry(metadata);
    if let Some(geometry) = geometry {
        ctx.with_area(geometry.border_box, |ctx| {
            ctx.render_widget(style.to_block());
        });
    } else {
        ctx.render_widget(Block::new().style(style.to_ratatui_style()));
    }
    let (markers, marker_width) = list_markers(items.len(), ordered_start);
    let area = geometry.map_or_else(|| ctx.area(), |geometry| geometry.content_box);

    ctx.with_area_inherited_style_and_selector_ancestor(
        area,
        style.inherited_values(),
        metadata.clone(),
        |ctx| {
            let bottom = area.y.saturating_add(area.height);
            let mut y = area.y;

            for (item, marker) in items.iter().zip(markers.iter()) {
                let remaining = bottom.saturating_sub(y);
                if remaining == 0 {
                    break;
                }

                let height =
                    cells_to_u16(measure_list_item(item, marker_width, ctx).height).min(remaining);
                let item_area = Rect { y, height, ..area };
                ctx.with_area(item_area, |ctx| {
                    render_marked_list_item(item, marker, marker_width, ctx)
                })?;
                y = y.saturating_add(height);
            }

            Ok(())
        },
    )
}

/// Renders one list item with its aligned marker.
///
/// # Arguments
///
/// * `item` — List item or fallback block view to render.
/// * `marker` — Marker text shown on the first item row.
/// * `marker_width` — Shared marker-column width for the containing list.
/// * `ctx` — Rendering context for this item.
///
/// # Returns
///
/// An empty [`Result`] on success.
///
/// # Errors
///
/// Returns [`crate::app::Error::Io`] if child rendering performs terminal I/O
/// that fails.
fn render_marked_list_item(
    item: &AnyView,
    marker: &str,
    marker_width: u16,
    ctx: &mut RenderCtx<'_, '_>,
) -> Result<()> {
    if let Some(item) = item.downcast_ref::<ListItemView>() {
        let style = resolve_style(&item.metadata, ctx);
        ctx.render_widget(Block::new().style(style.to_ratatui_style()));
        let area = ctx.area();
        return ctx.with_area_inherited_style_and_selector_ancestor(
            area,
            style.inherited_values(),
            item.metadata.clone(),
            |ctx| {
                render_list_item_marker(marker, marker_width, style, ctx);
                render_list_item_children(&item.children, marker_width, ctx)
            },
        );
    }

    render_list_item_marker(marker, marker_width, ctx.inherited_style(), ctx);
    render_list_item_children(std::slice::from_ref(item), marker_width, ctx)
}

/// Renders a right-aligned marker on the first row of the current item.
///
/// # Arguments
///
/// * `marker` — Marker text to render.
/// * `marker_width` — Shared width of the marker column.
/// * `style` — Resolved style applied to the marker.
/// * `ctx` — Rendering context for the item area.
fn render_list_item_marker(
    marker: &str,
    marker_width: u16,
    style: TuiStyle,
    ctx: &mut RenderCtx<'_, '_>,
) {
    let area = ctx.area();
    if area.width == 0 || area.height == 0 || marker_width == 0 {
        return;
    }

    let marker_area = Rect {
        width: marker_width.min(area.width),
        height: 1,
        ..area
    };
    let content = format!("{marker:>width$}", width = usize::from(marker_width));
    ctx.with_area(marker_area, |ctx| {
        ctx.render_widget(Paragraph::new(content).style(style.to_ratatui_style()));
    });
}

/// Renders vertically stacked blocks within a marked list item.
///
/// Nested lists begin two cells from the item's list edge. Other blocks begin
/// after the containing list's marker column and separating space.
///
/// # Arguments
///
/// * `children` — Document blocks contained by the item.
/// * `marker_width` — Shared marker-column width for the containing list.
/// * `ctx` — Rendering context for the item content.
///
/// # Returns
///
/// An empty [`Result`] on success.
///
/// # Errors
///
/// Returns [`crate::app::Error::Io`] if child rendering performs terminal I/O
/// that fails.
fn render_list_item_children(
    children: &[AnyView],
    marker_width: u16,
    ctx: &mut RenderCtx<'_, '_>,
) -> Result<()> {
    let area = ctx.area();
    let bottom = area.y.saturating_add(area.height);
    let mut y = area.y;

    for child in children {
        let remaining = bottom.saturating_sub(y);
        if remaining == 0 {
            break;
        }

        let indent = list_item_child_indent(child, marker_width);
        let child_base = horizontal_inset(Rect { y, ..area }, indent);
        let height = ctx
            .with_area(child_base, |ctx| {
                cells_to_u16(measure_view(child.as_view(), ctx).height)
            })
            .min(remaining);
        if height == 0 {
            continue;
        }

        let child_area = Rect {
            height,
            ..child_base
        };
        ctx.with_area(child_area, |ctx| child.render(ctx))?;
        y = y.saturating_add(height);
    }

    Ok(())
}
