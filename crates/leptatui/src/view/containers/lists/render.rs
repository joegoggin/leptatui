//! Shared semantic-list rendering, measurement, and focus geometry.

use ratatui::{
    layout::Rect,
    widgets::{Block, Paragraph},
};

use crate::view::core::render::{VerticalSpan, resolve_style};
use crate::view::{AnyView, ListItemView, ListView, StyleMetadata};
use crate::{TuiStyle, app::Result, component::RenderCtx};

/// Horizontal indentation applied to each recursively nested list.
const LIST_NEST_INDENT: u16 = 2;

/// Returns the focused control's vertical span within a child view.
fn focused_control_span_for_view(
    view: &AnyView,
    ctx: &mut RenderCtx<'_, '_>,
) -> Option<VerticalSpan> {
    view.__focused_button_span(ctx)
        .map(|(top, bottom)| VerticalSpan { top, bottom })
}

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
                let item_height = min_height_for_list_item(item, marker_width, ctx);
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
        let child_height = ctx.with_area(child_base, |ctx| child.__min_height(ctx));
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
    ctx.render_widget(Block::new().style(style.to_ratatui_style()));
    let (markers, marker_width) = list_markers(items.len(), ordered_start);
    let area = ctx.area();

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

                let height = min_height_for_list_item(item, marker_width, ctx).min(remaining);
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

/// Returns marker strings and the widest marker width for a list.
///
/// # Arguments
///
/// * `item_count` — Number of markers to create.
/// * `ordered_start` — First decimal marker, or [`None`] for hyphen markers.
///
/// # Returns
///
/// A tuple containing marker strings and their maximum terminal width.
fn list_markers(item_count: usize, ordered_start: Option<usize>) -> (Vec<String>, u16) {
    let markers = (0..item_count)
        .map(|index| {
            ordered_start.map_or_else(
                || "-".to_owned(),
                |start| format!("{}.", start.saturating_add(index)),
            )
        })
        .collect::<Vec<_>>();
    let width = markers
        .iter()
        .map(String::len)
        .max()
        .and_then(|width| u16::try_from(width).ok())
        .unwrap_or(0);

    (markers, width)
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
            .with_area(child_base, |ctx| child.__min_height(ctx))
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

/// Returns the minimum render height for one marked list item.
///
/// # Arguments
///
/// * `item` — List item or fallback block view to measure.
/// * `marker_width` — Shared marker-column width for the containing list.
/// * `ctx` — Rendering context containing the available width.
///
/// # Returns
///
/// A [`u16`] height including a marker-only row for empty items.
fn min_height_for_list_item(item: &AnyView, marker_width: u16, ctx: &mut RenderCtx<'_, '_>) -> u16 {
    if let Some(item) = item.downcast_ref::<ListItemView>() {
        let style = resolve_style(&item.metadata, ctx);
        let area = ctx.area();
        return ctx.with_area_inherited_style_and_selector_ancestor(
            area,
            style.inherited_values(),
            item.metadata.clone(),
            |ctx| min_height_for_list_item_children(&item.children, marker_width, ctx),
        );
    }

    min_height_for_list_item_children(std::slice::from_ref(item), marker_width, ctx)
}

/// Returns the intrinsic height of an ordered or unordered list.
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
/// A [`u16`] sum of all item heights.
pub(crate) fn min_height_for_list_view(
    items: &[AnyView],
    ordered_start: Option<usize>,
    metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> u16 {
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
                .map(|item| min_height_for_list_item(item, marker_width, ctx))
                .fold(0, u16::saturating_add)
        },
    )
}

/// Returns the stacked height of blocks inside one marked list item.
///
/// # Arguments
///
/// * `children` — Document blocks contained by the item.
/// * `marker_width` — Shared marker-column width for the containing list.
/// * `ctx` — Rendering context containing the available width.
///
/// # Returns
///
/// A [`u16`] height of at least one row for the item marker.
fn min_height_for_list_item_children(
    children: &[AnyView],
    marker_width: u16,
    ctx: &mut RenderCtx<'_, '_>,
) -> u16 {
    let area = ctx.area();
    children
        .iter()
        .map(|child| {
            let indent = list_item_child_indent(child, marker_width);
            let child_area = horizontal_inset(area, indent);
            ctx.with_area(child_area, |ctx| child.__min_height(ctx))
        })
        .fold(0, u16::saturating_add)
        .max(1)
}

/// Returns the horizontal offset for a list-item child block.
///
/// # Arguments
///
/// * `child` — Child view whose semantic role selects the indentation.
/// * `marker_width` — Shared marker-column width for the containing list.
///
/// # Returns
///
/// A [`u16`] indentation in terminal cells.
fn list_item_child_indent(child: &AnyView, marker_width: u16) -> u16 {
    if child.is::<ListView>() {
        LIST_NEST_INDENT
    } else {
        marker_width.saturating_add(1)
    }
}

/// Insets a rectangle horizontally without underflowing narrow areas.
///
/// # Arguments
///
/// * `area` — Rectangle to inset.
/// * `indent` — Requested number of cells to remove from the left edge.
///
/// # Returns
///
/// A [`Rect`] narrowed by the available indentation.
fn horizontal_inset(area: Rect, indent: u16) -> Rect {
    let applied = indent.min(area.width);
    Rect {
        x: area.x.saturating_add(applied),
        width: area.width.saturating_sub(applied),
        ..area
    }
}
