//! Vim word-motion helpers.

use super::cursor::{
    char_at, next_char_boundary, normal_cursor, normal_last_char_cursor, previous_char_boundary,
};

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
