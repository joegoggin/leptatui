//! UTF-8 cursor boundaries and normal-mode cursor normalization.

use super::super::state::VimMode;

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
