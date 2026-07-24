//! Computed container rendering, measurement, scrolling, and focus geometry.

use ratatui::{
    layout::Rect,
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::view::core::render::{VerticalSpan, resolve_style, scroll_span_into_view};
use crate::view::{AnyView, StyleMetadata};
use crate::{TuiStyle, app::Result, component::RenderCtx};

/// Returns the focused control's vertical span inside computed child geometry.
///
/// # Arguments
///
/// * `children` — Child views searched for the focused control.
/// * `metadata` — Container metadata supplying computed content geometry.
/// * `ctx` — Render context used to reproduce inherited style and selector scopes.
///
/// # Returns
///
/// An optional [`VerticalSpan`] relative to the container content box.
pub(crate) fn focused_control_span_for_container(
    children: &[AnyView],
    metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> Option<VerticalSpan> {
    let style = resolve_style(metadata, ctx);
    let (content_area, layout_offset) = container_content_area(metadata, ctx.area());
    ctx.with_area_inherited_style_and_selector_ancestor(
        content_area,
        style.inherited_values(),
        metadata.clone(),
        |ctx| {
            children.iter().find_map(|child| {
                let child_area = child_area(child, content_area, layout_offset, ctx);
                let offset = u32::from(child_area.y.saturating_sub(content_area.y));
                ctx.with_area(child_area, |ctx| {
                    focused_control_span_for_view(child, ctx).map(|span| span.offset_by(offset))
                })
            })
        },
    )
}

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
    let style = resolve_style(metadata, ctx);
    ctx.record_metadata_hit_area(metadata);
    ctx.render_widget(style.to_block());

    let (content_area, layout_offset) = container_content_area(metadata, ctx.area());
    let content_height = children
        .iter()
        .map(|child| {
            child_area(child, content_area, layout_offset, ctx)
                .bottom()
                .saturating_sub(content_area.y)
        })
        .max()
        .unwrap_or(content_area.height);
    let max_scroll_offset = content_height.saturating_sub(content_area.height);
    metadata.set_max_scroll_offset(max_scroll_offset);

    if max_scroll_offset == 0 {
        return render_children(
            children,
            content_area,
            0,
            style.inherited_values(),
            metadata,
            layout_offset,
            ctx,
        );
    }

    if let Some(span) = focused_control_span_for_container(children, metadata, ctx) {
        let scroll_to_anchor = children.iter().any(AnyView::__has_scroll_to_anchor_request);
        if scroll_to_anchor {
            metadata.set_scroll_offset(
                u16::try_from(span.top.min(u32::from(max_scroll_offset))).unwrap_or(u16::MAX),
            );
        } else {
            scroll_span_into_view(metadata, span, content_area.height, max_scroll_offset);
        }
    }

    let row_offset = metadata.scroll_offset().min(max_scroll_offset);
    render_children(
        children,
        content_area,
        row_offset,
        style.inherited_values(),
        metadata,
        layout_offset,
        ctx,
    )?;
    render_scrollbar(row_offset, max_scroll_offset, content_area, ctx);
    Ok(())
}

/// Returns the legacy minimum height of block-flow children.
///
/// # Arguments
///
/// * `children` — Child views measured in source order.
/// * `metadata` — Container metadata supplying inherited style.
/// * `ctx` — Render context used for child measurement.
///
/// # Returns
///
/// A minimum terminal height covering every child.
pub(crate) fn min_height_for_container(
    children: &[AnyView],
    metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> u16 {
    let style = resolve_style(metadata, ctx);
    let area = ctx.area();
    ctx.with_area_inherited_style_and_selector_ancestor(
        area,
        style.inherited_values(),
        metadata.clone(),
        |ctx| {
            let heights = children.iter().map(|child| child.__min_height(ctx));
            if style.display == Some(crate::Display::Flex)
                && matches!(
                    style.flex_direction.unwrap_or_default(),
                    crate::FlexDirection::Row | crate::FlexDirection::RowReverse
                )
            {
                heights.max().unwrap_or(0)
            } else {
                heights.fold(0, u16::saturating_add)
            }
        },
    )
}

/// Returns the focused span for one child view.
///
/// # Arguments
///
/// * `view` — Child view searched for the focused control or scroll anchor.
/// * `ctx` — Render context defining the child's retained area.
///
/// # Returns
///
/// The focused control's vertical span when the child contains one.
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

/// Renders visible children into computed areas with an optional vertical offset.
///
/// # Arguments
///
/// * `children` — Child views rendered in source order.
/// * `content_area` — Parent content box used for clipping.
/// * `row_offset` — Vertical scroll offset applied to retained child geometry.
/// * `inherited_style` — Cascaded style inherited by each child.
/// * `parent_metadata` — Parent metadata supplying selector ancestry.
/// * `layout_offset` — Signed translation from retained to assigned geometry.
/// * `ctx` — Render context targeting the container.
///
/// # Returns
///
/// An empty [`Result`] on success.
///
/// # Errors
///
/// Returns [`crate::Error::Io`] if child rendering performs terminal I/O that fails.
fn render_children(
    children: &[AnyView],
    content_area: Rect,
    row_offset: u16,
    inherited_style: TuiStyle,
    parent_metadata: &StyleMetadata,
    layout_offset: (i32, i32),
    ctx: &mut RenderCtx<'_, '_>,
) -> Result<()> {
    for child in children {
        if child
            .style_metadata()
            .is_some_and(StyleMetadata::is_layout_hidden)
        {
            continue;
        }

        let full_area = child_area(child, content_area, layout_offset, ctx);
        let shifted_top = i32::from(full_area.y) - i32::from(row_offset);
        let shifted_bottom = shifted_top.saturating_add(i32::from(full_area.height));
        let visible_top = shifted_top.max(i32::from(content_area.y));
        let visible_bottom = shifted_bottom.min(i32::from(content_area.bottom()));
        if visible_bottom <= visible_top {
            continue;
        }
        let source_y = u16::try_from(visible_top.saturating_sub(shifted_top)).unwrap_or(u16::MAX);
        let shifted_area = Rect {
            y: u16::try_from(shifted_top.max(0)).unwrap_or(u16::MAX),
            ..full_area
        };
        let visible_area = Rect {
            x: full_area.x.max(content_area.x),
            y: u16::try_from(visible_top).unwrap_or(u16::MAX),
            width: full_area
                .right()
                .min(content_area.right())
                .saturating_sub(full_area.x.max(content_area.x)),
            height: u16::try_from(visible_bottom.saturating_sub(visible_top)).unwrap_or(u16::MAX),
        };

        if source_y == 0 && visible_area == shifted_area {
            ctx.with_assigned_area_inherited_style_and_selector_ancestor(
                shifted_area,
                inherited_style,
                parent_metadata.clone(),
                |ctx| child.as_view().render(ctx),
            )?;
        } else {
            ctx.render_view_clipped(
                child,
                full_area,
                source_y,
                visible_area,
                inherited_style,
                parent_metadata.clone(),
            )?;
        }
    }
    Ok(())
}

/// Returns one child's retained border box or the parent content fallback.
///
/// # Arguments
///
/// * `child` — Child view whose retained geometry is queried.
/// * `fallback` — Parent content box used when no child geometry is retained.
/// * `ctx` — Render context used to visit layout-transparent descendants.
///
/// # Returns
///
/// The union of the child's effective retained border boxes, or `fallback`.
fn child_area(
    child: &AnyView,
    fallback: Rect,
    layout_offset: (i32, i32),
    ctx: &mut RenderCtx<'_, '_>,
) -> Rect {
    if let Some(geometry) = ctx.unstyled_layout_geometry(child.as_view()) {
        return translate_rect(geometry.border_box, layout_offset);
    }
    if let Some(area) = child
        .style_metadata()
        .and_then(StyleMetadata::layout_geometry)
        .map(|geometry| geometry.border_box)
    {
        return translate_rect(area, layout_offset);
    }

    let mut area = None;
    child
        .as_view()
        .__visit_layout_children(ctx, &mut |nested, nested_ctx| {
            let nested_area = child_area(nested, fallback, layout_offset, nested_ctx);
            area = Some(area.map_or(nested_area, |current: Rect| current.union(nested_area)));
        });
    area.unwrap_or(fallback)
}

/// Returns translated content geometry and its retained-to-assigned offset.
///
/// # Arguments
///
/// * `metadata` — Container metadata supplying retained geometry.
/// * `assigned_area` — Area currently assigned by the render parent.
///
/// # Returns
///
/// The assigned content box and signed translation applied to descendants.
fn container_content_area(metadata: &StyleMetadata, assigned_area: Rect) -> (Rect, (i32, i32)) {
    let Some(geometry) = metadata.layout_geometry() else {
        return (assigned_area, (0, 0));
    };
    let offset = (
        i32::from(assigned_area.x) - i32::from(geometry.border_box.x),
        i32::from(assigned_area.y) - i32::from(geometry.border_box.y),
    );
    (translate_rect(geometry.content_box, offset), offset)
}

/// Translates a terminal rectangle by signed cell offsets.
///
/// # Arguments
///
/// * `area` — Retained rectangle to translate.
/// * `offset` — Signed horizontal and vertical cell offsets.
///
/// # Returns
///
/// A rectangle whose origin is clamped to terminal coordinate bounds.
fn translate_rect(area: Rect, offset: (i32, i32)) -> Rect {
    Rect {
        x: u16::try_from((i32::from(area.x) + offset.0).clamp(0, i32::from(u16::MAX)))
            .unwrap_or(u16::MAX),
        y: u16::try_from((i32::from(area.y) + offset.1).clamp(0, i32::from(u16::MAX)))
            .unwrap_or(u16::MAX),
        ..area
    }
}

/// Renders a vertical scrollbar for overflowing computed content.
///
/// # Arguments
///
/// * `row_offset` — Current vertical scroll offset.
/// * `max_scroll_offset` — Largest permitted vertical scroll offset.
/// * `content_area` — Container content box that receives the scrollbar.
/// * `ctx` — Render context targeting the container.
fn render_scrollbar(
    row_offset: u16,
    max_scroll_offset: u16,
    content_area: Rect,
    ctx: &mut RenderCtx<'_, '_>,
) {
    if content_area.width == 0 || content_area.height == 0 {
        return;
    }

    let mut state = ScrollbarState::new(usize::from(max_scroll_offset).saturating_add(1))
        .position(usize::from(row_offset))
        .viewport_content_length(usize::from(content_area.height));
    ctx.with_area(content_area, |ctx| {
        ctx.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            &mut state,
        );
    });
}
