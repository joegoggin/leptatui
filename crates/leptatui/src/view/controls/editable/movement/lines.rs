//! Logical-line ranges and vertical text-area navigation.

use std::ops::Range;

use super::super::model::EditableControlKind;
use super::cursor::{clamp_cursor, previous_char_boundary};

/// Returns the logical line start for an editable control.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `cursor` — Cursor byte index used to select the line.
/// * `kind` — Editable control variant that defines line behavior.
///
/// # Returns
///
/// A [`usize`] byte index for the start of the logical line.
pub(crate) fn line_start(value: &str, cursor: usize, kind: EditableControlKind) -> usize {
    match kind {
        EditableControlKind::Input => 0,
        EditableControlKind::TextArea => text_area_line_start(value, cursor),
    }
}

/// Returns the insert-mode line end for an editable control.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `cursor` — Cursor byte index used to select the line.
/// * `kind` — Editable control variant that defines line behavior.
///
/// # Returns
///
/// A [`usize`] byte index for the insert-mode end of the logical line.
pub(crate) fn insert_line_end(value: &str, cursor: usize, kind: EditableControlKind) -> usize {
    match kind {
        EditableControlKind::Input => value.len(),
        EditableControlKind::TextArea => text_area_line_end(value, cursor),
    }
}

/// Returns the normal-mode cursor position for the end of the current line.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `cursor` — Cursor byte index used to select the line.
/// * `kind` — Editable control variant that defines line behavior.
///
/// # Returns
///
/// A [`usize`] byte index for the last character in the logical line, or the
/// line start for an empty line.
pub(crate) fn normal_line_end(value: &str, cursor: usize, kind: EditableControlKind) -> usize {
    let start = line_start(value, cursor, kind);
    let end = insert_line_end(value, cursor, kind);
    if end > start {
        previous_char_boundary(value, end)
    } else {
        start
    }
}

/// Returns the current text-area line content range without a trailing newline.
///
/// # Arguments
///
/// * `value` — Current controlled text-area value.
/// * `cursor` — Cursor byte index used to select the line.
///
/// # Returns
///
/// A [`Range`] covering the current line content.
pub(crate) fn text_area_line_content_range(value: &str, cursor: usize) -> Range<usize> {
    let start = text_area_line_start(value, cursor);
    let end = text_area_line_end(value, cursor);
    start..end
}

/// Returns the text-area range removed by a linewise delete.
///
/// # Arguments
///
/// * `value` — Current controlled text-area value.
/// * `cursor` — Cursor byte index used to select the line.
///
/// # Returns
///
/// A [`Range`] covering the bytes removed by a linewise delete.
pub(crate) fn text_area_line_delete_range(value: &str, cursor: usize) -> Range<usize> {
    let start = text_area_line_start(value, cursor);
    let end = text_area_line_end(value, cursor);

    if end < value.len() {
        start..end + 1
    } else if start > 0 {
        start - 1..end
    } else {
        start..end
    }
}

/// Returns the byte index at the start of the cursor's logical line.
///
/// # Arguments
///
/// * `value` — Text-area value containing logical lines.
/// * `cursor` — Cursor byte index used to select the line.
///
/// # Returns
///
/// A [`usize`] byte index for the start of the logical line.
pub(crate) fn text_area_line_start(value: &str, cursor: usize) -> usize {
    let cursor = clamp_cursor(value, cursor);
    value[..cursor].rfind('\n').map_or(0, |index| index + 1)
}

/// Returns the byte index at the end of the cursor's logical line.
///
/// # Arguments
///
/// * `value` — Text-area value containing logical lines.
/// * `cursor` — Cursor byte index used to select the line.
///
/// # Returns
///
/// A [`usize`] byte index for the end of the logical line.
pub(crate) fn text_area_line_end(value: &str, cursor: usize) -> usize {
    let cursor = clamp_cursor(value, cursor);
    value[cursor..]
        .find('\n')
        .map_or(value.len(), |index| cursor + index)
}

/// Returns the character column represented by a cursor within its logical line.
///
/// # Arguments
///
/// * `value` — Text-area value containing logical lines.
/// * `cursor` — Cursor byte index used to select the line and column.
///
/// # Returns
///
/// A [`usize`] character column within the logical line.
pub(crate) fn text_area_line_column(value: &str, cursor: usize) -> usize {
    let cursor = clamp_cursor(value, cursor);
    let start = text_area_line_start(value, cursor);
    value[start..cursor].chars().count()
}

/// Returns the cursor byte index for a character column inside a line range.
///
/// # Arguments
///
/// * `value` — Text-area value containing the target line.
/// * `line_start` — Byte index where the target line starts.
/// * `line_end` — Byte index where the target line ends.
/// * `target_column` — Character column to locate within the line.
///
/// # Returns
///
/// A [`usize`] cursor byte index for the target line and column.
pub(crate) fn text_area_cursor_for_line_column(
    value: &str,
    line_start: usize,
    line_end: usize,
    target_column: usize,
) -> usize {
    let mut column = 0usize;

    for (offset, _) in value[line_start..line_end].char_indices() {
        if column == target_column {
            return line_start + offset;
        }
        column = column.saturating_add(1);
    }

    line_end
}

/// Returns the cursor position on the previous logical line.
///
/// # Arguments
///
/// * `value` — Text-area value containing logical lines.
/// * `cursor` — Cursor byte index used to derive the source column.
///
/// # Returns
///
/// A [`usize`] cursor byte index on the previous line, or the original cursor
/// when no previous line exists.
pub(crate) fn text_area_previous_line_cursor(value: &str, cursor: usize) -> usize {
    let cursor = clamp_cursor(value, cursor);
    let current_start = text_area_line_start(value, cursor);
    if current_start == 0 {
        return cursor;
    }

    let target_column = text_area_line_column(value, cursor);
    let previous_end = current_start.saturating_sub(1);
    let previous_start = value[..previous_end]
        .rfind('\n')
        .map_or(0, |index| index + 1);

    text_area_cursor_for_line_column(value, previous_start, previous_end, target_column)
}

/// Returns the cursor position on the next logical line.
///
/// # Arguments
///
/// * `value` — Text-area value containing logical lines.
/// * `cursor` — Cursor byte index used to derive the source column.
///
/// # Returns
///
/// A [`usize`] cursor byte index on the next line, or the original cursor when
/// no next line exists.
pub(crate) fn text_area_next_line_cursor(value: &str, cursor: usize) -> usize {
    let cursor = clamp_cursor(value, cursor);
    let current_end = text_area_line_end(value, cursor);
    if current_end == value.len() {
        return cursor;
    }

    let target_column = text_area_line_column(value, cursor);
    let next_start = current_end + 1;
    let next_end = value[next_start..]
        .find('\n')
        .map_or(value.len(), |index| next_start + index);

    text_area_cursor_for_line_column(value, next_start, next_end, target_column)
}
