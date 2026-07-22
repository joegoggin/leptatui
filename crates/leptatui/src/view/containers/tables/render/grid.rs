//! Table border glyph construction and cell-alignment conversion.

use ratatui::{
    layout::Alignment,
    text::{Line, Text},
};

use crate::view::CellAlignment;

/// Creates one horizontal border line for a responsive table.
///
/// # Arguments
///
/// * `widths` — Visible content widths between border intersections.
/// * `position` — Whether the line is the top, middle, or bottom boundary.
///
/// # Returns
///
/// A [`String`] containing plain Unicode terminal border glyphs.
pub(super) fn table_border_line(widths: &[u16], position: TableBorderPosition) -> String {
    let (left, intersection, right) = match position {
        TableBorderPosition::Top => ('┌', '┬', '┐'),
        TableBorderPosition::Middle => ('├', '┼', '┤'),
        TableBorderPosition::Bottom => ('└', '┴', '┘'),
    };
    let mut line = String::from(left);
    for (index, width) in widths.iter().enumerate() {
        line.push_str(&"─".repeat(usize::from(*width)));
        line.push(if index + 1 == widths.len() {
            right
        } else {
            intersection
        });
    }

    line
}

/// Creates a vertical table border spanning a row's rendered height.
///
/// # Arguments
///
/// * `height` — Number of terminal rows to fill.
///
/// # Returns
///
/// A [`Text`] value containing one vertical border glyph per line.
pub(super) fn table_vertical_border(height: u16) -> Text<'static> {
    Text::from(vec![Line::raw("│"); usize::from(height)])
}

/// Position of a horizontal table border within the rendered grid.
#[derive(Clone, Copy)]
pub(super) enum TableBorderPosition {
    /// First boundary above all rows.
    Top,
    /// Shared boundary between two rows.
    Middle,
    /// Final boundary below all rows.
    Bottom,
}

/// Converts public table-cell alignment into Ratatui paragraph alignment.
///
/// # Arguments
///
/// * `alignment` — Public cell alignment value.
///
/// # Returns
///
/// The corresponding Ratatui [`Alignment`] value.
pub(super) fn ratatui_cell_alignment(alignment: CellAlignment) -> Alignment {
    match alignment {
        CellAlignment::Left => Alignment::Left,
        CellAlignment::Center => Alignment::Center,
        CellAlignment::Right => Alignment::Right,
    }
}
