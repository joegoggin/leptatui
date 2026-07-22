//! Character-wise and linewise paste transformations.

use super::super::visual::replace_value_range;
use super::{
    cursor::{insert_after_normal_cursor, normal_cursor_after_change},
    lines::text_area_line_end,
};

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
