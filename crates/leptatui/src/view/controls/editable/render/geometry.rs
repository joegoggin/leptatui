//! Viewport scrolling, wrapping, cursor placement, and focus geometry.

use super::super::{
    model::{EditableControlKind, EditableModel},
    movement::{char_column, clamp_cursor},
};
use crate::{
    Borders, RenderCtx,
    view::core::render::{VerticalSpan, resolve_style},
};
use ratatui::layout::{Position, Rect};

/// Returns the horizontal scroll offset needed to keep an input cursor visible.
///
/// # Arguments
///
/// * `value` — Input value used to map the cursor byte index to a character
///   column.
/// * `cursor` — Cursor byte index to keep visible.
/// * `current_scroll` — Current horizontal scroll offset.
/// * `width` — Available input render width.
///
/// # Returns
///
/// A [`u16`] scroll offset that keeps the cursor within the render width.
pub(super) fn input_horizontal_scroll(
    value: &str,
    cursor: usize,
    current_scroll: u16,
    width: u16,
) -> u16 {
    if width == 0 {
        return 0;
    }

    let cursor = clamp_cursor(value, cursor);
    let cursor_column = char_column(value, cursor);
    let width = usize::from(width);
    let current_scroll = usize::from(current_scroll);
    let next_scroll = if cursor_column < current_scroll {
        cursor_column
    } else if cursor_column > current_scroll.saturating_add(width) {
        cursor_column.saturating_sub(width)
    } else {
        current_scroll
    };

    u16::try_from(next_scroll).unwrap_or(u16::MAX)
}

/// Sets the terminal cursor for a focused single-line input.
pub(super) fn set_input_cursor(
    value: &str,
    cursor: usize,
    horizontal_scroll: u16,
    ctx: &mut RenderCtx<'_, '_>,
) {
    let area = ctx.area();
    if area.width == 0 || area.height == 0 {
        return;
    }

    let column = char_column(value, cursor).saturating_sub(usize::from(horizontal_scroll));
    ctx.set_cursor_position(cursor_position_in_area(area, column, 0));
}

/// Sets the terminal cursor for a focused multiline text area.
pub(super) fn set_text_area_cursor(
    value: &str,
    cursor: usize,
    vertical_scroll: u16,
    horizontal_scroll: u16,
    ctx: &mut RenderCtx<'_, '_>,
) {
    let area = ctx.area();
    if area.width == 0 || area.height == 0 {
        return;
    }

    let (row, column) = text_area_cursor_position(value, cursor, area.width);
    let row = row.saturating_sub(usize::from(vertical_scroll));
    let column = column.saturating_sub(usize::from(horizontal_scroll));
    ctx.set_cursor_position(cursor_position_in_area(area, column, row));
}

/// Sets the terminal cursor on a pending inserted text-area character.
pub(super) fn set_text_area_pending_insert_cursor(
    value: &str,
    cursor: usize,
    vertical_scroll: u16,
    horizontal_scroll: u16,
    ctx: &mut RenderCtx<'_, '_>,
) {
    let area = ctx.area();
    if area.width == 0 || area.height == 0 {
        return;
    }

    let (row, column) = text_area_character_position(value, cursor, area.width);
    let row = row.saturating_sub(usize::from(vertical_scroll));
    let column = column.saturating_sub(usize::from(horizontal_scroll));
    ctx.set_cursor_position(cursor_position_in_area(area, column, row));
}

/// Returns an absolute cursor position clamped inside a render area.
fn cursor_position_in_area(area: Rect, column: usize, row: usize) -> Position {
    Position {
        x: area.x.saturating_add(
            u16::try_from(column.min(usize::from(area.width.saturating_sub(1))))
                .unwrap_or(u16::MAX),
        ),
        y: area.y.saturating_add(
            u16::try_from(row.min(usize::from(area.height.saturating_sub(1)))).unwrap_or(u16::MAX),
        ),
    }
}

/// Returns the vertical scroll offset needed to keep a text-area cursor visible.
///
/// # Arguments
///
/// * `value` — Text-area value used to map the cursor byte index to a rendered
///   row.
/// * `cursor` — Cursor byte index to keep visible.
/// * `current_scroll` — Current vertical scroll offset.
/// * `height` — Available text-area render height.
/// * `width` — Available text-area render width.
///
/// # Returns
///
/// A [`u16`] scroll offset that keeps the cursor within the render height.
pub(super) fn text_area_vertical_scroll(
    value: &str,
    cursor: usize,
    current_scroll: u16,
    height: u16,
    width: u16,
) -> u16 {
    if height == 0 || width == 0 {
        return 0;
    }

    let viewport_height = usize::from(height);
    let total_rows = text_area_rendered_rows(value, width);
    let max_scroll = total_rows.saturating_sub(viewport_height);
    let current_scroll = usize::from(current_scroll).min(max_scroll);
    let cursor_row = text_area_cursor_row(value, cursor, width);
    let viewport_bottom = current_scroll.saturating_add(viewport_height);
    let next_scroll = if cursor_row < current_scroll {
        cursor_row
    } else if cursor_row >= viewport_bottom {
        cursor_row.saturating_sub(viewport_height.saturating_sub(1))
    } else {
        current_scroll
    }
    .min(max_scroll);

    u16::try_from(next_scroll).unwrap_or(u16::MAX)
}

/// Returns the number of wrapped rows needed to render a text-area value.
///
/// # Arguments
///
/// * `value` — Text-area value to measure.
/// * `width` — Available text-area render width.
///
/// # Returns
///
/// A [`usize`] row count for the wrapped text-area value.
fn text_area_rendered_rows(value: &str, width: u16) -> usize {
    if width == 0 {
        return 1;
    }

    let width = usize::from(width);
    let mut rows = 1usize;
    let mut column = 0usize;

    for character in value.chars() {
        if character == '\n' {
            rows = rows.saturating_add(1);
            column = 0;
            continue;
        }

        if column == width {
            rows = rows.saturating_add(1);
            column = 0;
        }
        column = column.saturating_add(1);
    }

    rows
}

/// Returns the wrapped render row represented by a text-area cursor.
///
/// # Arguments
///
/// * `value` — Text-area value used to map the cursor byte index.
/// * `cursor` — Cursor byte index to locate.
/// * `width` — Available text-area render width.
///
/// # Returns
///
/// A [`usize`] row index containing the cursor.
fn text_area_cursor_row(value: &str, cursor: usize, width: u16) -> usize {
    text_area_cursor_position(value, cursor, width).0
}

/// Returns the wrapped render row and column represented by a text-area cursor.
fn text_area_cursor_position(value: &str, cursor: usize, width: u16) -> (usize, usize) {
    if width == 0 {
        return (0, 0);
    }

    let cursor = clamp_cursor(value, cursor);
    let width = usize::from(width);
    let mut row = 0usize;
    let mut column = 0usize;

    for (index, character) in value.char_indices() {
        if index >= cursor {
            break;
        }

        if character == '\n' {
            row = row.saturating_add(1);
            column = 0;
            continue;
        }

        if column == width {
            row = row.saturating_add(1);
            column = 0;
        }
        column = column.saturating_add(1);
    }

    (row, column)
}

/// Returns the wrapped render row and column for the character at a byte index.
fn text_area_character_position(value: &str, cursor: usize, width: u16) -> (usize, usize) {
    if width == 0 {
        return (0, 0);
    }

    let cursor = clamp_cursor(value, cursor);
    let width = usize::from(width);
    let mut row = 0usize;
    let mut column = 0usize;

    for (index, character) in value.char_indices() {
        if index >= cursor {
            if index == cursor && character != '\n' && column == width {
                row = row.saturating_add(1);
                column = 0;
            }
            break;
        }

        if character == '\n' {
            row = row.saturating_add(1);
            column = 0;
            continue;
        }

        if column == width {
            row = row.saturating_add(1);
            column = 0;
        }
        column = column.saturating_add(1);
    }

    (row, column)
}

/// Returns the focus visibility span for an editable text control.
pub(crate) fn focused_control_span_for_editor(
    view: &EditableModel,
    ctx: &mut RenderCtx<'_, '_>,
) -> Option<VerticalSpan> {
    if !view.metadata.is_focused() || !view.metadata.scroll_into_view_requested() {
        return None;
    }
    if view.kind == EditableControlKind::Input {
        return Some(VerticalSpan {
            top: 0,
            bottom: u32::from(ctx.area().height),
        });
    }

    let style = resolve_style(&view.metadata, ctx);
    let area = ctx.area();
    let inner = style
        .to_block_with_default_borders(Borders::ALL)
        .inner(area);
    let top_offset = u32::from(inner.y.saturating_sub(area.y));
    let cursor_row = u32::try_from(text_area_cursor_row(
        &view.value,
        view.editable_state.cursor(),
        inner.width,
    ))
    .unwrap_or(u32::MAX);
    let top = top_offset.saturating_add(cursor_row);
    Some(VerticalSpan {
        top,
        bottom: top.saturating_add(1),
    })
}
