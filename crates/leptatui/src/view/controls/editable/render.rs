//! Shared rendering and editing engine for editable controls.

use std::{ops::Range, time::Instant};

use super::{
    insert::insert_key_pending_expired,
    model::{EditableControlKind, EditableModel},
    movement::*,
    state::{EditableState, VimMode},
    visual::visual_selection_range,
};
use crate::view::core::render::{
    line_count_height, resolve_style, vertical_border_rows, vertical_padding_rows,
};
use crate::{Borders, Modifier, Result, TuiStyle, component::RenderCtx};
use ratatui::{
    layout::{Position, Rect},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};

/// Returns extra empty rows dropped by string-backed paragraph conversion.
fn trailing_text_area_empty_line_rows(value: &str) -> usize {
    usize::from(value.ends_with('\n'))
}

fn input_paragraph<'a>(
    value: &'a str,
    style: TuiStyle,
    horizontal_scroll: u16,
    selection: Option<Range<usize>>,
) -> Paragraph<'a> {
    let paragraph = selection.map_or_else(
        || Paragraph::new(value),
        |selection| Paragraph::new(selected_text_lines(value, selection, style)),
    );

    paragraph
        .style(style.to_ratatui_style())
        .scroll((0, horizontal_scroll))
}

/// Returns a paragraph configured for multiline editable control rendering.
///
/// # Arguments
///
/// * `value` — Text value to render inside the text area.
/// * `style` — Resolved view style applied to the paragraph.
/// * `vertical_scroll` — Vertical viewport offset applied to the paragraph.
/// * `horizontal_scroll` — Horizontal viewport offset applied to the paragraph.
///
/// # Returns
///
/// A [`Paragraph`] configured for text-area rendering.
fn text_area_paragraph<'a>(
    value: &'a str,
    style: TuiStyle,
    vertical_scroll: u16,
    horizontal_scroll: u16,
    selection: Option<Range<usize>>,
) -> Paragraph<'a> {
    let paragraph = selection.map_or_else(
        || Paragraph::new(value),
        |selection| Paragraph::new(selected_text_lines(value, selection, style)),
    );

    paragraph
        .style(style.to_ratatui_style())
        .wrap(Wrap { trim: false })
        .scroll((vertical_scroll, horizontal_scroll))
}

/// Returns logical text lines with the selected bytes rendered in reverse video.
fn selected_text_lines<'a>(
    value: &'a str,
    selection: Range<usize>,
    style: TuiStyle,
) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    let mut line_start = 0usize;
    let selection_style = style.to_ratatui_style().add_modifier(Modifier::REVERSED);

    loop {
        let line_end = value[line_start..]
            .find('\n')
            .map_or(value.len(), |index| line_start + index);
        lines.push(Line::from(selected_line_spans(
            value,
            line_start..line_end,
            selection.clone(),
            selection_style,
        )));

        if line_end == value.len() {
            break;
        }

        line_start = line_end + 1;
        if line_start == value.len() {
            lines.push(Line::from(Vec::<Span<'a>>::new()));
            break;
        }
    }

    lines
}

/// Returns spans for one logical line with the selection split out.
fn selected_line_spans<'a>(
    value: &'a str,
    line: Range<usize>,
    selection: Range<usize>,
    selection_style: ratatui::style::Style,
) -> Vec<Span<'a>> {
    if selection.start == selection.end
        || selection.end <= line.start
        || selection.start >= line.end
    {
        return vec![Span::raw(&value[line])];
    }

    let selected_start = selection.start.max(line.start);
    let selected_end = selection.end.min(line.end);
    let mut spans = Vec::new();

    if line.start < selected_start {
        spans.push(Span::raw(&value[line.start..selected_start]));
    }
    spans.push(Span::styled(
        &value[selected_start..selected_end],
        selection_style,
    ));
    if selected_end < line.end {
        spans.push(Span::raw(&value[selected_end..line.end]));
    }

    spans
}

/// Converts a rendered line count into a saturated terminal height.
/// Renders a controlled input or text area.
pub(crate) fn render_editable_text_view(
    view: &EditableModel,
    ctx: &mut RenderCtx<'_, '_>,
) -> Result<()> {
    let style = resolve_style(&view.metadata, ctx);
    let block = style.to_block_with_default_borders(Borders::ALL);
    let inner = block.inner(ctx.area());
    let pending = pending_insert_render(&view.value, &view.editable_state);
    let display_value = if let Some(pending) = pending.as_ref() {
        pending.value.as_str()
    } else if view.value.is_empty() {
        view.placeholder.as_deref().unwrap_or("")
    } else {
        view.value.as_str()
    };
    ctx.render_widget(block);

    match view.kind {
        EditableControlKind::Input => {
            let horizontal_scroll = if let Some(pending) = pending.as_ref() {
                input_horizontal_scroll(
                    &pending.value,
                    pending.scroll_cursor,
                    view.editable_state.horizontal_scroll(),
                    inner.width,
                )
            } else if view.value.is_empty() {
                0
            } else {
                input_horizontal_scroll(
                    &view.value,
                    view.editable_state.cursor(),
                    view.editable_state.horizontal_scroll(),
                    inner.width,
                )
            };
            ctx.with_area(inner, |ctx| {
                ctx.render_widget(input_paragraph(
                    display_value,
                    style,
                    horizontal_scroll,
                    pending
                        .as_ref()
                        .and_then(|pending| pending.selection.clone())
                        .or_else(|| {
                            visual_selection_range(
                                &view.value,
                                &view.editable_state,
                                EditableControlKind::Input,
                            )
                        }),
                ));
                if view.metadata.is_focused() {
                    let (value, cursor) = pending.as_ref().map_or(
                        (view.value.as_str(), view.editable_state.cursor()),
                        |pending| (pending.value.as_str(), pending.cursor),
                    );
                    set_input_cursor(value, cursor, horizontal_scroll, ctx);
                }
            });
        }
        EditableControlKind::TextArea => {
            let vertical_scroll = if let Some(pending) = pending.as_ref() {
                text_area_vertical_scroll(
                    &pending.value,
                    pending.scroll_cursor,
                    view.editable_state.vertical_scroll(),
                    inner.height,
                    inner.width,
                )
            } else if view.value.is_empty() {
                0
            } else {
                text_area_vertical_scroll(
                    &view.value,
                    view.editable_state.cursor(),
                    view.editable_state.vertical_scroll(),
                    inner.height,
                    inner.width,
                )
            };
            ctx.with_area(inner, |ctx| {
                ctx.render_widget(text_area_paragraph(
                    display_value,
                    style,
                    vertical_scroll,
                    view.editable_state.horizontal_scroll(),
                    pending
                        .as_ref()
                        .and_then(|pending| pending.selection.clone())
                        .or_else(|| {
                            visual_selection_range(
                                &view.value,
                                &view.editable_state,
                                EditableControlKind::TextArea,
                            )
                        }),
                ));
                if view.metadata.is_focused() {
                    if let Some(pending) = pending.as_ref() {
                        set_text_area_pending_insert_cursor(
                            &pending.value,
                            pending.cursor,
                            vertical_scroll,
                            view.editable_state.horizontal_scroll(),
                            ctx,
                        );
                    } else {
                        set_text_area_cursor(
                            &view.value,
                            view.editable_state.cursor(),
                            vertical_scroll,
                            view.editable_state.horizontal_scroll(),
                            ctx,
                        );
                    }
                }
            });
        }
    }
    view.metadata.clear_scroll_into_view_request();
    Ok(())
}

/// Returns the intrinsic height of a controlled input or text area.
pub(crate) fn min_height_for_editable_text_view(
    view: &EditableModel,
    ctx: &mut RenderCtx<'_, '_>,
) -> u16 {
    let style = resolve_style(&view.metadata, ctx);
    let border_height = vertical_border_rows(style.borders.unwrap_or(Borders::ALL));
    let padding_height = vertical_padding_rows(style.padding);
    if view.kind == EditableControlKind::Input {
        return 1u16
            .saturating_add(border_height)
            .saturating_add(padding_height);
    }

    let display_value = if view.value.is_empty() {
        view.placeholder.as_deref().unwrap_or("")
    } else {
        &view.value
    };
    let inner = style
        .to_block_with_default_borders(Borders::ALL)
        .inner(ctx.area());
    line_count_height(
        text_area_paragraph(display_value, style, 0, 0, None)
            .line_count(inner.width)
            .saturating_add(trailing_text_area_empty_line_rows(display_value)),
    )
    .max(1)
    .saturating_add(border_height)
    .saturating_add(padding_height)
}
/// Transient render state for an uncommitted pending insert-mode key.
struct PendingInsertRender {
    /// Display value with the pending key inserted.
    value: String,
    /// Byte range highlighted while the pending key is still active.
    selection: Option<Range<usize>>,
    /// Cursor byte index used to place the terminal cursor.
    cursor: usize,
    /// Cursor byte index used to scroll the pending key into view.
    scroll_cursor: usize,
}

/// Returns render-only display state for a pending insert-mode key.
fn pending_insert_render(
    value: &str,
    editable_state: &EditableState,
) -> Option<PendingInsertRender> {
    let pending = editable_state.insert_key_pending()?;
    if editable_state.mode() != VimMode::Insert {
        return None;
    }

    let now = Instant::now();
    let active = !insert_key_pending_expired(pending, now);
    let cursor = clamp_cursor(value, editable_state.cursor());
    let pending_key = pending.key();
    let mut display_value =
        String::with_capacity(value.len().saturating_add(pending_key.len_utf8()));
    display_value.push_str(&value[..cursor]);
    display_value.push(pending_key);
    display_value.push_str(&value[cursor..]);

    let pending_end = cursor.saturating_add(pending_key.len_utf8());
    Some(PendingInsertRender {
        value: display_value,
        selection: active.then_some(cursor..pending_end),
        cursor: if active { cursor } else { pending_end },
        scroll_cursor: pending_end,
    })
}

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
fn input_horizontal_scroll(value: &str, cursor: usize, current_scroll: u16, width: u16) -> u16 {
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
fn set_input_cursor(
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
fn set_text_area_cursor(
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
fn set_text_area_pending_insert_cursor(
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
fn text_area_vertical_scroll(
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

/// Vertical content span, with an exclusive bottom row.
#[derive(Clone, Copy)]
pub(crate) struct VerticalSpan {
    /// First row occupied by the span.
    top: u32,
    /// Row after the span.
    bottom: u32,
}

impl VerticalSpan {
    /// Creates a span starting at row zero with the provided height.
    fn from_height(height: u16) -> Self {
        Self {
            top: 0,
            bottom: u32::from(height),
        }
    }

    /// Converts the span to its tuple representation for hidden component APIs.
    pub(crate) fn into_tuple(self) -> (u32, u32) {
        (self.top, self.bottom)
    }
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
        return Some(VerticalSpan::from_height(ctx.area().height));
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
