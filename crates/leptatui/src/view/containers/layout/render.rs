//! Shared layout rendering, measurement, scrolling, and focus geometry.

use ratatui::{
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::view::core::render::{VerticalSpan, resolve_style, scroll_span_into_view};
use crate::view::{AnyView, StyleMetadata};
use crate::{
    app::Result,
    component::RenderCtx,
    style::{LayoutDirection, TuiStyle},
};

/// Returns the focused control's vertical span within a child view.
fn focused_control_span_for_view(
    view: &AnyView,
    ctx: &mut RenderCtx<'_, '_>,
) -> Option<VerticalSpan> {
    if view
        .style_metadata()
        .is_some_and(StyleMetadata::scroll_to_anchor_requested)
    {
        return Some(VerticalSpan {
            top: 0,
            bottom: u32::from(ctx.area().height),
        });
    }

    view.__focused_button_span(ctx)
        .map(|(top, bottom)| VerticalSpan { top, bottom })
}

pub(crate) fn focused_control_span_for_layout_view(
    children: &[AnyView],
    metadata: &StyleMetadata,
    default_direction: LayoutDirection,
    ctx: &mut RenderCtx<'_, '_>,
) -> Option<VerticalSpan> {
    if children.is_empty() {
        return None;
    }

    let style = resolve_style(metadata, ctx);
    let direction = style.direction.unwrap_or(default_direction);

    match direction {
        LayoutDirection::Row => {
            focused_control_span_in_row_children(children, style.inherited_values(), metadata, ctx)
        }
        LayoutDirection::Column => {
            let min_heights = child_min_heights(children, style.inherited_values(), metadata, ctx);
            focused_control_span_in_column_children(
                children,
                &min_heights,
                style.inherited_values(),
                metadata,
                ctx,
            )
        }
    }
}

/// Returns the focused control's vertical span inside row children.
fn focused_control_span_in_row_children(
    children: &[AnyView],
    inherited_style: TuiStyle,
    parent_metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> Option<VerticalSpan> {
    let area = ctx.area();
    let constraints = vec![Constraint::Fill(1); children.len()];
    let areas = Layout::horizontal(constraints).split(area);

    ctx.with_area_inherited_style_and_selector_ancestor(
        area,
        inherited_style,
        parent_metadata.clone(),
        |ctx| {
            children.iter().zip(areas.iter()).find_map(|(child, area)| {
                ctx.with_area(*area, |ctx| focused_control_span_for_view(child, ctx))
            })
        },
    )
}

/// Returns the focused control's vertical span inside column children.
fn focused_control_span_in_column_children(
    children: &[AnyView],
    min_heights: &[u16],
    inherited_style: TuiStyle,
    parent_metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> Option<VerticalSpan> {
    let area = ctx.area();

    ctx.with_area_inherited_style_and_selector_ancestor(
        area,
        inherited_style,
        parent_metadata.clone(),
        |ctx| {
            let mut row = 0u32;

            for (child, min_height) in children.iter().zip(min_heights.iter()) {
                let child_area = Rect {
                    height: *min_height,
                    ..area
                };

                if let Some(span) =
                    ctx.with_area(child_area, |ctx| focused_control_span_for_view(child, ctx))
                {
                    return Some(span.offset_by(row));
                }

                row = row.saturating_add(u32::from(*min_height));
            }

            None
        },
    )
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
pub(crate) fn render_layout_view(
    children: &[AnyView],
    metadata: &StyleMetadata,
    default_direction: LayoutDirection,
    ctx: &mut RenderCtx<'_, '_>,
) -> Result<()> {
    let style = resolve_style(metadata, ctx);
    ctx.record_metadata_hit_area(metadata);
    ctx.render_widget(Block::new().style(style.to_ratatui_style()));
    render_children(
        children,
        style.direction.unwrap_or(default_direction),
        style.inherited_values(),
        metadata,
        ctx,
    )
}

/// Renders child views into row or column areas.
///
/// # Arguments
///
/// * `children` — Views to render into split areas.
/// * `direction` — Axis used to split the current context area.
/// * `inherited_style` — Style values inherited by child views.
/// * `parent_metadata` — Metadata to append to each child's selector ancestor
///   path.
/// * `ctx` — Rendering context for the parent area.
///
/// # Returns
///
/// An empty [`Result`] on success.
///
/// # Errors
///
/// Returns [`crate::app::Error::Io`] if child rendering performs terminal I/O
/// that fails.
fn render_children(
    children: &[AnyView],
    direction: LayoutDirection,
    inherited_style: TuiStyle,
    parent_metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> Result<()> {
    if children.is_empty() {
        parent_metadata.set_max_scroll_offset(0);
        return Ok(());
    }

    if direction == LayoutDirection::Column
        && try_render_overflowing_column_children(children, inherited_style, parent_metadata, ctx)?
    {
        return Ok(());
    }

    parent_metadata.set_max_scroll_offset(0);

    let constraints = child_constraints(children, direction, inherited_style, parent_metadata, ctx);
    let areas = match direction {
        LayoutDirection::Row => Layout::horizontal(constraints).split(ctx.area()),
        LayoutDirection::Column => Layout::vertical(constraints).split(ctx.area()),
    };

    for (child, area) in children.iter().zip(areas.iter()) {
        ctx.with_area_inherited_style_and_selector_ancestor(
            *area,
            inherited_style,
            parent_metadata.clone(),
            |ctx| child.render(ctx),
        )?;
    }

    Ok(())
}

/// Renders a vertically overflowing column when the children exceed the viewport.
fn try_render_overflowing_column_children(
    children: &[AnyView],
    inherited_style: TuiStyle,
    parent_metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> Result<bool> {
    let min_heights = child_min_heights(children, inherited_style, parent_metadata, ctx);
    let content_height: u32 = min_heights.iter().map(|height| u32::from(*height)).sum();
    let area = ctx.area();
    let area_height = area.height;

    if content_height <= u32::from(area_height) || area_height == 0 {
        return Ok(false);
    }

    let content_area = scrolled_content_area(area);
    let min_heights = ctx.with_area(content_area, |ctx| {
        child_min_heights(children, inherited_style, parent_metadata, ctx)
    });
    let scrolled_content_height: u32 = min_heights.iter().map(|height| u32::from(*height)).sum();
    let content_height = scrolled_content_height.max(content_height);
    let max_scroll_offset =
        u16::try_from(content_height.saturating_sub(u32::from(area_height))).unwrap_or(u16::MAX);
    parent_metadata.set_max_scroll_offset(max_scroll_offset);

    let scroll_to_anchor = children.iter().any(AnyView::__has_scroll_to_anchor_request);

    if let Some(span) = ctx.with_area(content_area, |ctx| {
        focused_control_span_in_column_children(
            children,
            &min_heights,
            inherited_style,
            parent_metadata,
            ctx,
        )
    }) {
        if scroll_to_anchor {
            let top = span.top.min(u32::from(max_scroll_offset));
            parent_metadata.set_scroll_offset(u16::try_from(top).unwrap_or(u16::MAX));
        } else {
            scroll_span_into_view(parent_metadata, span, area_height, max_scroll_offset);
        }
    }

    let row_offset = parent_metadata.scroll_offset().min(max_scroll_offset);
    ctx.with_area(content_area, |ctx| {
        render_scrolled_column_children(
            children,
            &min_heights,
            row_offset,
            inherited_style,
            parent_metadata,
            ctx,
        )
    })?;
    render_column_scrollbar(row_offset, max_scroll_offset, area_height, ctx);

    Ok(true)
}

/// Returns the content area used when a right-side scrollbar is visible.
fn scrolled_content_area(area: Rect) -> Rect {
    Rect {
        width: area.width.saturating_sub(1),
        ..area
    }
}

/// Renders the right-side scrollbar for an overflowing column.
fn render_column_scrollbar(
    row_offset: u16,
    max_scroll_offset: u16,
    viewport_height: u16,
    ctx: &mut RenderCtx<'_, '_>,
) {
    if ctx.area().width == 0 || viewport_height == 0 {
        return;
    }

    let content_length = usize::from(max_scroll_offset).saturating_add(1);
    let mut state = ScrollbarState::new(content_length)
        .position(usize::from(row_offset))
        .viewport_content_length(usize::from(viewport_height));

    ctx.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None),
        &mut state,
    );
}

/// Renders a vertically overflowing column from a child scroll offset.
fn render_scrolled_column_children(
    children: &[AnyView],
    min_heights: &[u16],
    row_offset: u16,
    inherited_style: TuiStyle,
    parent_metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> Result<()> {
    let area = ctx.area();
    let bottom = area.y.saturating_add(area.height);
    let mut y = area.y;
    let mut skipped_rows = row_offset;

    for (child, min_height) in children.iter().zip(min_heights.iter()) {
        if skipped_rows >= *min_height {
            skipped_rows -= *min_height;
            continue;
        }

        let remaining = bottom.saturating_sub(y);
        if remaining == 0 {
            break;
        }

        let source_y = skipped_rows;
        skipped_rows = 0;
        let height = min_height.saturating_sub(source_y).min(remaining);
        if height == 0 {
            continue;
        }

        let child_area = Rect {
            x: area.x,
            y,
            width: area.width,
            height,
        };

        if source_y == 0 && height == *min_height {
            ctx.with_area_inherited_style_and_selector_ancestor(
                child_area,
                inherited_style,
                parent_metadata.clone(),
                |ctx| child.render(ctx),
            )?;
        } else {
            ctx.render_view_clipped(
                child,
                Rect {
                    x: area.x,
                    y,
                    width: area.width,
                    height: *min_height,
                },
                source_y,
                child_area,
                inherited_style,
                parent_metadata.clone(),
            )?;
        }

        y = y.saturating_add(height);
    }

    Ok(())
}

/// Returns constraints for child layout.
fn child_constraints(
    children: &[AnyView],
    direction: LayoutDirection,
    inherited_style: TuiStyle,
    parent_metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> Vec<Constraint> {
    if direction == LayoutDirection::Column {
        let min_heights = child_min_heights(children, inherited_style, parent_metadata, ctx);
        if min_heights.iter().any(|height| *height > 1) {
            return min_heights.into_iter().map(Constraint::Min).collect();
        }
    }

    vec![Constraint::Fill(1); children.len()]
}

/// Returns minimum render heights for child views in a parent selector scope.
fn child_min_heights(
    children: &[AnyView],
    inherited_style: TuiStyle,
    parent_metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> Vec<u16> {
    let area = ctx.area();
    ctx.with_area_inherited_style_and_selector_ancestor(
        area,
        inherited_style,
        parent_metadata.clone(),
        |ctx| {
            children
                .iter()
                .map(|child| child.__min_height(ctx))
                .collect()
        },
    )
}

/// Returns child minimum heights after applying row split widths.
fn row_child_min_heights(
    children: &[AnyView],
    inherited_style: TuiStyle,
    parent_metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> Vec<u16> {
    let area = ctx.area();
    let constraints = vec![Constraint::Fill(1); children.len()];
    let areas = Layout::horizontal(constraints).split(area);

    ctx.with_area_inherited_style_and_selector_ancestor(
        area,
        inherited_style,
        parent_metadata.clone(),
        |ctx| {
            children
                .iter()
                .zip(areas.iter())
                .map(|(child, area)| ctx.with_area(*area, |ctx| child.__min_height(ctx)))
                .collect()
        },
    )
}

/// Returns minimum height for a layout view after resolving its direction.
pub(crate) fn min_height_for_layout_view(
    children: &[AnyView],
    metadata: &StyleMetadata,
    default_direction: LayoutDirection,
    ctx: &mut RenderCtx<'_, '_>,
) -> u16 {
    if children.is_empty() {
        return 0;
    }

    let style = resolve_style(metadata, ctx);
    let direction = style.direction.unwrap_or(default_direction);
    let min_heights = match direction {
        LayoutDirection::Row => {
            row_child_min_heights(children, style.inherited_values(), metadata, ctx)
        }
        LayoutDirection::Column => {
            child_min_heights(children, style.inherited_values(), metadata, ctx)
        }
    };

    match direction {
        LayoutDirection::Row => min_heights.into_iter().max().unwrap_or(0),
        LayoutDirection::Column => min_heights.into_iter().fold(0, u16::saturating_add),
    }
}
