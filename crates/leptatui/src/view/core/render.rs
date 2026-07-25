//! Shared rendering and vertical-geometry helpers.

use ratatui::{
    text::Text,
    widgets::{Paragraph, Wrap},
};

use crate::view::AnyView;
use crate::{RenderCtx, TuiSpacing, TuiStyle};

use super::metadata::StyleMetadata;

/// Resolves a view style from the current render context.
pub(crate) fn resolve_style(metadata: &StyleMetadata, ctx: &RenderCtx<'_, '_>) -> TuiStyle {
    ctx.resolve_style(metadata)
}

/// Creates a wrapped paragraph for semantic rich text.
pub(crate) fn semantic_paragraph(content: &Text<'static>, style: TuiStyle) -> Paragraph<'static> {
    Paragraph::new(content.clone())
        .style(style.to_ratatui_style())
        .wrap(Wrap { trim: false })
}

/// Converts a line count into a saturated terminal height.
pub(crate) fn line_count_height(line_count: usize) -> u16 {
    u16::try_from(line_count).unwrap_or(u16::MAX)
}

/// Returns how many rows the configured borders consume.
pub(crate) fn vertical_border_rows(borders: crate::Borders) -> u16 {
    u16::from(borders.contains(crate::Borders::TOP))
        + u16::from(borders.contains(crate::Borders::BOTTOM))
}

/// Returns how many columns the configured borders consume.
///
/// # Arguments
///
/// * `borders` — Border edges applied to a view.
///
/// # Returns
///
/// A `u16` count containing the horizontal border columns.
pub(crate) fn horizontal_border_columns(borders: crate::Borders) -> u16 {
    u16::from(borders.contains(crate::Borders::LEFT))
        + u16::from(borders.contains(crate::Borders::RIGHT))
}

/// Returns how many rows the configured padding consumes.
pub(crate) fn vertical_padding_rows(padding: Option<TuiSpacing>) -> u16 {
    padding.map_or(0, |padding| padding.top.saturating_add(padding.bottom))
}

/// Returns how many columns the configured padding consumes.
///
/// # Arguments
///
/// * `padding` — Optional physical padding values.
///
/// # Returns
///
/// A `u16` count containing the horizontal padding columns.
pub(crate) fn horizontal_padding_columns(padding: Option<TuiSpacing>) -> u16 {
    padding.map_or(0, |padding| padding.left.saturating_add(padding.right))
}

/// Vertical content span with an exclusive bottom row.
#[derive(Clone, Copy)]
pub(crate) struct VerticalSpan {
    /// First row occupied by the span.
    pub(crate) top: u32,
    /// Row after the span.
    pub(crate) bottom: u32,
}

impl VerticalSpan {
    /// Returns this span offset by a parent content row.
    pub(crate) fn offset_by(self, offset: u32) -> Self {
        Self {
            top: self.top.saturating_add(offset),
            bottom: self.bottom.saturating_add(offset),
        }
    }

    /// Returns the span height.
    pub(crate) fn height(self) -> u32 {
        self.bottom.saturating_sub(self.top)
    }

    /// Converts this span to the hidden view-contract tuple representation.
    pub(crate) fn into_tuple(self) -> (u32, u32) {
        (self.top, self.bottom)
    }
}

/// Returns the focused control's vertical span within a child view.
///
/// # Arguments
///
/// * `view` — Child view searched for the focused control.
/// * `ctx` — Render context defining the child's retained area.
///
/// # Returns
///
/// An optional [`VerticalSpan`] when the child contains a focused control.
pub(crate) fn focused_control_span_for_view(
    view: &AnyView,
    ctx: &mut RenderCtx<'_, '_>,
) -> Option<VerticalSpan> {
    view.__focused_button_span(ctx)
        .map(|(top, bottom)| VerticalSpan { top, bottom })
}

/// Moves a scroll offset just enough to make a span visible.
pub(crate) fn scroll_span_into_view(
    metadata: &StyleMetadata,
    span: VerticalSpan,
    viewport_height: u16,
    max_scroll_offset: u16,
) {
    if viewport_height == 0 {
        return;
    }

    let viewport_height = u32::from(viewport_height);
    let current = u32::from(metadata.scroll_offset().min(max_scroll_offset));
    let viewport_bottom = current.saturating_add(viewport_height);
    let next = if span.top < current {
        span.top
    } else if span.bottom > viewport_bottom {
        if span.height() > viewport_height {
            span.top
        } else {
            span.bottom.saturating_sub(viewport_height)
        }
    } else {
        current
    }
    .min(u32::from(max_scroll_offset));

    metadata.set_scroll_offset(u16::try_from(next).unwrap_or(u16::MAX));
}
