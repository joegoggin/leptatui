//! UTF-8-safe cursor, word, line, and text transformation helpers.

use std::ops::Range;

use super::visual::replace_value_range;
use super::{model::EditableControlKind, state::VimMode};

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

/// Converts an insert-mode cursor to the matching normal-mode cursor.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `cursor` — Insert-mode cursor byte index.
///
/// # Returns
///
/// A [`usize`] byte index for the normal-mode cursor.
pub(crate) fn normal_cursor_from_insert(value: &str, cursor: usize) -> usize {
    let cursor = clamp_cursor(value, cursor);
    if cursor == 0 || is_trailing_empty_line_cursor(value, cursor) {
        cursor
    } else {
        previous_char_boundary(value, cursor)
    }
}

/// Returns a normal-mode cursor clamped to an existing normal-mode position.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `cursor` — Candidate cursor byte index.
///
/// # Returns
///
/// A [`usize`] byte index for an existing character, the trailing empty line, or
/// zero for an empty value.
pub(crate) fn normal_cursor(value: &str, cursor: usize) -> usize {
    if value.is_empty() {
        return 0;
    }

    let cursor = clamp_cursor(value, cursor);
    if cursor == value.len() && !is_trailing_empty_line_cursor(value, cursor) {
        previous_char_boundary(value, cursor)
    } else {
        cursor
    }
}

/// Returns the final normal-mode cursor position in a value.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
///
/// # Returns
///
/// A [`usize`] byte index for the final character, the trailing empty line, or
/// zero for an empty value.
pub(crate) fn normal_last_char_cursor(value: &str) -> usize {
    normal_cursor(value, value.len())
}

/// Returns whether a cursor addresses the empty logical line after a final newline.
pub(crate) fn is_trailing_empty_line_cursor(value: &str, cursor: usize) -> bool {
    cursor == value.len() && value.ends_with('\n')
}

/// Returns the previous normal-mode character cursor.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `cursor` — Cursor byte index used as the movement origin.
///
/// # Returns
///
/// A [`usize`] byte index for the previous normal-mode cursor.
pub(crate) fn normal_previous_char_cursor(value: &str, cursor: usize) -> usize {
    previous_char_boundary(value, normal_cursor(value, cursor))
}

/// Returns the next normal-mode character cursor.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `cursor` — Cursor byte index used as the movement origin.
///
/// # Returns
///
/// A [`usize`] byte index for the next normal-mode cursor.
pub(crate) fn normal_next_char_cursor(value: &str, cursor: usize) -> usize {
    if value.is_empty() {
        return 0;
    }

    let cursor = normal_cursor(value, cursor);
    let next = next_char_boundary(value, cursor);
    if next == value.len() { cursor } else { next }
}

/// Returns the insert position after the current normal-mode cursor.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `cursor` — Normal-mode cursor byte index.
///
/// # Returns
///
/// A [`usize`] byte index where inserted text should begin.
pub(crate) fn insert_after_normal_cursor(value: &str, cursor: usize) -> usize {
    if value.is_empty() {
        0
    } else {
        next_char_boundary(value, normal_cursor(value, cursor))
    }
}

/// Returns a cursor after replacing the controlled value.
///
/// # Arguments
///
/// * `value` — Replacement controlled editable value.
/// * `cursor` — Cursor byte index retained before replacement.
/// * `mode` — Vim mode that determines cursor clamping behavior.
///
/// # Returns
///
/// A [`usize`] byte index valid for the replacement value.
pub(crate) fn cursor_after_value_replace(value: &str, cursor: usize, mode: VimMode) -> usize {
    match mode {
        VimMode::Insert => clamp_cursor(value, cursor),
        VimMode::Normal | VimMode::Visual | VimMode::VisualLine => {
            normal_cursor_after_change(value, cursor)
        }
    }
}

/// Returns a normal-mode cursor after mutating text near `cursor`.
///
/// # Arguments
///
/// * `value` — Mutated controlled editable value.
/// * `cursor` — Cursor byte index near the mutation.
///
/// # Returns
///
/// A [`usize`] normal-mode cursor byte index valid for the mutated value.
pub(crate) fn normal_cursor_after_change(value: &str, cursor: usize) -> usize {
    normal_cursor(value, cursor)
}

/// Returns the character at a byte cursor.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `cursor` — Cursor byte index to inspect.
///
/// # Returns
///
/// An [`Option`] containing the character starting at `cursor`.
pub(crate) fn char_at(value: &str, cursor: usize) -> Option<char> {
    value.get(cursor..)?.chars().next()
}

/// Returns whether a character participates in Vim word motions.
///
/// # Arguments
///
/// * `character` — Character to classify.
///
/// # Returns
///
/// A [`bool`] value indicating whether `character` belongs to a Vim word.
pub(crate) fn is_word_character(character: char) -> bool {
    !character.is_whitespace()
}

/// Returns the start of the next word for normal-mode `w`.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `cursor` — Cursor byte index used as the movement origin.
///
/// # Returns
///
/// A [`usize`] byte index for the next word start.
pub(crate) fn next_word_start_cursor(value: &str, cursor: usize) -> usize {
    if value.is_empty() {
        return 0;
    }

    let mut cursor = normal_cursor(value, cursor);
    if char_at(value, cursor).is_some_and(is_word_character) {
        while cursor < value.len() && char_at(value, cursor).is_some_and(is_word_character) {
            cursor = next_char_boundary(value, cursor);
        }
    }

    while cursor < value.len()
        && char_at(value, cursor).is_some_and(|character| !is_word_character(character))
    {
        cursor = next_char_boundary(value, cursor);
    }

    if cursor == value.len() {
        normal_last_char_cursor(value)
    } else {
        cursor
    }
}

/// Returns the start of the previous word for normal-mode `b`.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `cursor` — Cursor byte index used as the movement origin.
///
/// # Returns
///
/// A [`usize`] byte index for the previous word start.
pub(crate) fn previous_word_start_cursor(value: &str, cursor: usize) -> usize {
    if value.is_empty() {
        return 0;
    }

    let cursor = normal_cursor(value, cursor);
    if cursor == 0 {
        return 0;
    }

    let mut cursor = previous_char_boundary(value, cursor);
    while cursor > 0
        && char_at(value, cursor).is_some_and(|character| !is_word_character(character))
    {
        cursor = previous_char_boundary(value, cursor);
    }

    while cursor > 0 {
        let previous = previous_char_boundary(value, cursor);
        if char_at(value, previous).is_some_and(|character| !is_word_character(character)) {
            break;
        }
        cursor = previous;
    }

    cursor
}

/// Returns the end of the current or next word for normal-mode `e`.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `cursor` — Cursor byte index used as the movement origin.
///
/// # Returns
///
/// A [`usize`] byte index for the current or next word end.
pub(crate) fn word_end_cursor(value: &str, cursor: usize) -> usize {
    if value.is_empty() {
        return 0;
    }

    let mut cursor = normal_cursor(value, cursor);
    if char_at(value, cursor).is_some_and(is_word_character) {
        let next = next_char_boundary(value, cursor);
        if next < value.len() && char_at(value, next).is_some_and(is_word_character) {
            cursor = next;
            while next_char_boundary(value, cursor) < value.len()
                && char_at(value, next_char_boundary(value, cursor)).is_some_and(is_word_character)
            {
                cursor = next_char_boundary(value, cursor);
            }
            return cursor;
        }
        cursor = next;
    }

    while cursor < value.len()
        && char_at(value, cursor).is_some_and(|character| !is_word_character(character))
    {
        cursor = next_char_boundary(value, cursor);
    }

    if cursor == value.len() {
        return normal_last_char_cursor(value);
    }

    while next_char_boundary(value, cursor) < value.len()
        && char_at(value, next_char_boundary(value, cursor)).is_some_and(is_word_character)
    {
        cursor = next_char_boundary(value, cursor);
    }

    cursor
}

/// Returns a charwise paste result and normal-mode cursor.
///
/// # Arguments
///
/// * `value` — Current controlled editable value.
/// * `cursor` — Normal-mode cursor byte index used as the paste origin.
/// * `yank_buffer` — Character-wise yank buffer to insert.
///
/// # Returns
///
/// A `(String, usize)` tuple containing the pasted value and next
/// normal-mode cursor.
pub(crate) fn charwise_paste(value: &str, cursor: usize, yank_buffer: &str) -> (String, usize) {
    let insert_at = insert_after_normal_cursor(value, cursor);
    let next = replace_value_range(value, insert_at..insert_at, yank_buffer);
    let next_cursor =
        normal_cursor_after_change(&next, insert_at.saturating_add(yank_buffer.len()));

    (next, next_cursor)
}

/// Returns a linewise text-area paste result and normal-mode cursor.
///
/// # Arguments
///
/// * `value` — Current controlled text-area value.
/// * `cursor` — Normal-mode cursor byte index used to select the current line.
/// * `yank_buffer` — Linewise yank buffer to insert.
///
/// # Returns
///
/// A `(String, usize)` tuple containing the pasted value and next
/// normal-mode cursor.
pub(crate) fn text_area_linewise_paste(
    value: &str,
    cursor: usize,
    yank_buffer: &str,
) -> (String, usize) {
    if value.is_empty() {
        return (yank_buffer.to_owned(), 0);
    }

    let current_end = text_area_line_end(value, cursor);
    if current_end < value.len() {
        let insert_at = current_end + 1;
        let mut replacement = String::with_capacity(yank_buffer.len().saturating_add(1));
        replacement.push_str(yank_buffer);
        replacement.push('\n');
        let next = replace_value_range(value, insert_at..insert_at, &replacement);
        return (next, insert_at);
    }

    let insert_at = value.len().saturating_add(1);
    let mut next = String::with_capacity(
        value
            .len()
            .saturating_add(yank_buffer.len())
            .saturating_add(1),
    );
    next.push_str(value);
    next.push('\n');
    next.push_str(yank_buffer);
    let next_cursor = normal_cursor_after_change(&next, insert_at);

    (next, next_cursor)
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

/// Clamps a cursor to a valid byte index and UTF-8 character boundary.
///
/// # Arguments
///
/// * `value` — Input value that defines valid byte boundaries.
/// * `cursor` — Candidate cursor byte index.
///
/// # Returns
///
/// A [`usize`] cursor byte index within `value` and on a UTF-8 boundary.
pub(crate) fn clamp_cursor(value: &str, cursor: usize) -> usize {
    let mut cursor = cursor.min(value.len());
    while !value.is_char_boundary(cursor) {
        cursor = cursor.saturating_sub(1);
    }

    cursor
}

/// Returns the previous character boundary before or at a cursor.
///
/// # Arguments
///
/// * `value` — Input value that defines valid byte boundaries.
/// * `cursor` — Candidate cursor byte index.
///
/// # Returns
///
/// A [`usize`] cursor byte index for the previous character boundary.
pub(crate) fn previous_char_boundary(value: &str, cursor: usize) -> usize {
    let cursor = clamp_cursor(value, cursor);
    value[..cursor]
        .char_indices()
        .next_back()
        .map_or(0, |(index, _)| index)
}

/// Returns the next character boundary after or at a cursor.
///
/// # Arguments
///
/// * `value` — Input value that defines valid byte boundaries.
/// * `cursor` — Candidate cursor byte index.
///
/// # Returns
///
/// A [`usize`] cursor byte index for the next character boundary.
pub(crate) fn next_char_boundary(value: &str, cursor: usize) -> usize {
    let cursor = clamp_cursor(value, cursor);
    if cursor == value.len() {
        return cursor;
    }

    value[cursor..]
        .char_indices()
        .nth(1)
        .map_or(value.len(), |(index, _)| cursor + index)
}

/// Returns the character column represented by a byte cursor.
///
/// # Arguments
///
/// * `value` — Input value that defines character columns.
/// * `cursor` — Candidate cursor byte index.
///
/// # Returns
///
/// A [`usize`] character column represented by the clamped cursor.
pub(crate) fn char_column(value: &str, cursor: usize) -> usize {
    let cursor = clamp_cursor(value, cursor);
    value[..cursor].chars().count()
}
