//! Shared marker and indentation geometry for semantic lists.

use ratatui::layout::Rect;

use crate::view::{AnyView, ListView};

/// Horizontal indentation applied to each recursively nested list.
const LIST_NEST_INDENT: u16 = 2;

/// Returns marker strings and the widest marker width for a list.
///
/// # Arguments
///
/// * `item_count` — Number of markers to create.
/// * `ordered_start` — First decimal marker, or [`None`] for hyphen markers.
///
/// # Returns
///
/// A tuple containing marker strings and their maximum terminal width.
pub(super) fn list_markers(item_count: usize, ordered_start: Option<usize>) -> (Vec<String>, u16) {
    let markers = (0..item_count)
        .map(|index| {
            ordered_start.map_or_else(
                || "-".to_owned(),
                |start| format!("{}.", start.saturating_add(index)),
            )
        })
        .collect::<Vec<_>>();
    let width = markers
        .iter()
        .map(String::len)
        .max()
        .and_then(|width| u16::try_from(width).ok())
        .unwrap_or(0);

    (markers, width)
}

/// Returns the horizontal offset for a list-item child block.
///
/// # Arguments
///
/// * `child` — Child view whose semantic role selects the indentation.
/// * `marker_width` — Shared marker-column width for the containing list.
///
/// # Returns
///
/// A [`u16`] indentation in terminal cells.
pub(super) fn list_item_child_indent(child: &AnyView, marker_width: u16) -> u16 {
    if child.is::<ListView>() {
        LIST_NEST_INDENT
    } else {
        marker_width.saturating_add(1)
    }
}

/// Insets a rectangle horizontally without underflowing narrow areas.
///
/// # Arguments
///
/// * `area` — Rectangle to inset.
/// * `indent` — Requested number of cells to remove from the left edge.
///
/// # Returns
///
/// A [`Rect`] narrowed by the available indentation.
pub(super) fn horizontal_inset(area: Rect, indent: u16) -> Rect {
    let applied = indent.min(area.width);
    Rect {
        x: area.x.saturating_add(applied),
        width: area.width.saturating_sub(applied),
        ..area
    }
}
