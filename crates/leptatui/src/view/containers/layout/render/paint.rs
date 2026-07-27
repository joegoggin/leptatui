//! Ordered child painting, clipping, and scrollbar rendering.

use ratatui::{
    layout::Rect,
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::view::{AnyView, StyleMetadata};
use crate::{Axes, Position, TuiStyle, app::Result, component::RenderCtx};

use super::{
    geometry::{child_geometry, local_geometry, scroll_geometry},
    positioning::{child_paint_style, positioned_child_origin, translated_geometry},
};

/// Geometry and clipping settings used while painting container children.
#[derive(Clone, Copy)]
pub(super) struct ChildPaintOptions {
    /// Parent content box used for retained child positioning.
    pub(super) content_area: Rect,
    /// Accumulated parent clip applied to each direct child.
    pub(super) clip: Rect,
    /// Signed translation from retained to assigned geometry.
    pub(super) layout_offset: (i32, i32),
}

/// Renders visible children into computed areas with two-axis scroll offsets.
///
/// # Arguments
///
/// * `children` — Child views rendered in source order.
/// * `offsets` — Horizontal and vertical offsets applied to child geometry.
/// * `inherited_style` — Cascaded style inherited by each child.
/// * `parent_metadata` — Parent metadata supplying selector ancestry.
/// * `options` — Retained-geometry translation and axis clipping settings.
/// * `ctx` — Render context targeting the container.
///
/// # Returns
///
/// An empty [`Result`] on success.
///
/// # Errors
///
/// Returns [`crate::Error::Io`] if child rendering performs terminal I/O that fails.
pub(super) fn render_children(
    children: &[AnyView],
    offsets: Axes<u16>,
    inherited_style: TuiStyle,
    parent_metadata: &StyleMetadata,
    options: ChildPaintOptions,
    ctx: &mut RenderCtx<'_, '_>,
) -> Result<()> {
    let mut paint_order = children
        .iter()
        .enumerate()
        .map(|(source_index, child)| {
            let (position, stacking_level, insets) = ctx
                .with_area_inherited_style_and_selector_ancestor(
                    options.content_area,
                    inherited_style.clone(),
                    parent_metadata.clone(),
                    |child_ctx| child_paint_style(child, child_ctx),
                );
            (stacking_level, source_index, position, insets, child)
        })
        .collect::<Vec<_>>();
    paint_order
        .sort_by_key(|(stacking_level, source_index, _, _, _)| (*stacking_level, *source_index));

    for (_, _, position, insets, child) in paint_order {
        if child
            .style_metadata()
            .is_some_and(StyleMetadata::is_layout_hidden)
            || position == Position::Fixed
        {
            continue;
        }

        let geometry = child_geometry(
            child,
            options.content_area,
            options.clip,
            options.layout_offset,
            ctx,
        );
        let full_area = geometry.border_box;
        let (shifted_left, shifted_top) = positioned_child_origin(
            full_area,
            offsets,
            position,
            ctx.sticky_scrollport(),
            insets,
            ctx.viewport_size(),
        );
        let shifted_right = shifted_left.saturating_add(i32::from(full_area.width));
        let shifted_bottom = shifted_top.saturating_add(i32::from(full_area.height));
        let visible_top = shifted_top.max(i32::from(options.clip.y));
        let visible_bottom = shifted_bottom.min(i32::from(options.clip.bottom()));
        let visible_left = shifted_left.max(i32::from(options.clip.x));
        let visible_right = shifted_right.min(i32::from(options.clip.right()));
        if visible_bottom <= visible_top || visible_right <= visible_left {
            continue;
        }
        let source_x = u16::try_from(visible_left.saturating_sub(shifted_left)).unwrap_or(u16::MAX);
        let source_y = u16::try_from(visible_top.saturating_sub(shifted_top)).unwrap_or(u16::MAX);
        let shifted_area = Rect {
            x: u16::try_from(shifted_left.max(0)).unwrap_or(u16::MAX),
            y: u16::try_from(shifted_top.max(0)).unwrap_or(u16::MAX),
            ..full_area
        };
        let visible_area = Rect {
            x: u16::try_from(visible_left).unwrap_or(u16::MAX),
            y: u16::try_from(visible_top).unwrap_or(u16::MAX),
            width: u16::try_from(visible_right.saturating_sub(visible_left)).unwrap_or(u16::MAX),
            height: u16::try_from(visible_bottom.saturating_sub(visible_top)).unwrap_or(u16::MAX),
        };
        if source_x == 0 && source_y == 0 && visible_area == shifted_area {
            let geometry = if position == Position::Sticky {
                translated_geometry(geometry, shifted_left, shifted_top)
            } else {
                scroll_geometry(geometry, offsets)
            };
            ctx.with_assigned_layout_geometry_and_selector_ancestor(
                geometry,
                child.style_metadata(),
                inherited_style.clone(),
                parent_metadata.clone(),
                |ctx| child.as_view().render(ctx),
            )?;
        } else {
            let geometry = local_geometry(
                geometry,
                Rect::new(source_x, source_y, visible_area.width, visible_area.height),
            );
            ctx.render_view_clipped(
                child,
                geometry,
                ratatui::layout::Position::new(source_x, source_y),
                visible_area,
                inherited_style.clone(),
                parent_metadata.clone(),
            )?;
        }
    }
    Ok(())
}

/// Renders visible horizontal and vertical scrollbars.
///
/// # Arguments
///
/// * `offsets` — Current horizontal and vertical scroll offsets.
/// * `maximum` — Largest permitted offsets on both axes.
/// * `content_area` — Container content box that receives the scrollbar.
/// * `viewport` — Final content viewport excluding scrollbar gutters.
/// * `gutters` — Whether horizontal and vertical gutters are visible.
/// * `ctx` — Render context targeting the container.
pub(super) fn render_scrollbars(
    offsets: Axes<u16>,
    maximum: Axes<u16>,
    content_area: Rect,
    viewport: Rect,
    gutters: Axes<bool>,
    ctx: &mut RenderCtx<'_, '_>,
) {
    if content_area.width == 0 || content_area.height == 0 {
        return;
    }

    if gutters.y {
        let area = Rect {
            height: viewport.height,
            ..content_area
        };
        let mut state = ScrollbarState::new(usize::from(maximum.y).saturating_add(1))
            .position(usize::from(offsets.y))
            .viewport_content_length(usize::from(area.height));
        ctx.with_area(area, |ctx| {
            ctx.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .end_symbol(None),
                &mut state,
            );
        });
    }
    if gutters.x {
        let area = Rect {
            width: viewport.width,
            ..content_area
        };
        let mut state = ScrollbarState::new(usize::from(maximum.x).saturating_add(1))
            .position(usize::from(offsets.x))
            .viewport_content_length(usize::from(area.width));
        ctx.with_area(area, |ctx| {
            ctx.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
                    .begin_symbol(None)
                    .end_symbol(None),
                &mut state,
            );
        });
    }
}
