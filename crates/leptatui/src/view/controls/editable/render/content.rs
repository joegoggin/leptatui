//! Paragraph, selection, and pending-insert display construction.

use std::{ops::Range, time::Instant};

use super::super::{
    insert::insert_key_pending_expired,
    movement::clamp_cursor,
    state::{EditableState, VimMode},
};
use crate::{Modifier, TuiStyle};
use ratatui::{
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};

/// Returns extra empty rows dropped by string-backed paragraph conversion.
///
/// # Arguments
///
/// * `value` — Text-area value to inspect.
///
/// # Returns
///
/// A [`usize`] count containing one row for a trailing newline or zero.
pub(super) fn trailing_text_area_empty_line_rows(value: &str) -> usize {
    usize::from(value.ends_with('\n'))
}

/// Returns a single-line paragraph with scrolling and selection styling.
///
/// # Arguments
///
/// * `value` — Input value to render.
/// * `style` — Resolved terminal style for the input.
/// * `horizontal_scroll` — Horizontal viewport offset.
/// * `selection` — Optional selected byte range to render in reverse video.
///
/// # Returns
///
/// A [`Paragraph`] configured for single-line input rendering.
pub(super) fn input_paragraph<'a>(
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
/// * `selection` — Optional selected byte range to render in reverse video.
///
/// # Returns
///
/// A [`Paragraph`] configured for text-area rendering.
pub(super) fn text_area_paragraph<'a>(
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
///
/// # Arguments
///
/// * `value` — Text value to split into logical lines.
/// * `selection` — Selected byte range to highlight.
/// * `style` — Resolved terminal style used as the selection base.
///
/// # Returns
///
/// A [`Vec`] containing one styled [`Line`] per logical input line.
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
///
/// # Arguments
///
/// * `value` — Complete text value containing the line.
/// * `line` — Byte range occupied by the logical line.
/// * `selection` — Selected byte range within the complete value.
/// * `selection_style` — Ratatui style applied to selected bytes.
///
/// # Returns
///
/// A [`Vec`] containing unselected and selected spans for the line.
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

/// Transient render state for an uncommitted pending insert-mode key.
pub(super) struct PendingInsertRender {
    /// Display value with the pending key inserted.
    pub(super) value: String,
    /// Byte range highlighted while the pending key is still active.
    pub(super) selection: Option<Range<usize>>,
    /// Cursor byte index used to place the terminal cursor.
    pub(super) cursor: usize,
    /// Cursor byte index used to scroll the pending key into view.
    pub(super) scroll_cursor: usize,
}

/// Returns render-only display state for a pending insert-mode key.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `editable_state` — Editing state containing any pending insert key.
///
/// # Returns
///
/// An optional [`PendingInsertRender`] when an insert key is pending in insert
/// mode.
pub(super) fn pending_insert_render(
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
