//! Markdown table conversion.

use pulldown_cmark::{Alignment, Event, Tag, TagEnd};

use crate::{
    AnyView, CellAlignment, IntoView, table, table_body, table_cell, table_head, table_row,
};

use super::inline_events::parse_inline;

/// Parses a CommonMark table into semantic header and body sections.
///
/// Pulldown-cmark emits header cells directly inside `TableHead`, so this
/// conversion creates the semantic header row that Leptatui tables require.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned after the table opening tag.
/// * `alignments` — Parsed alignment for each source column.
///
/// # Returns
///
/// A semantic table containing one header section and one body section.
pub(super) fn parse_table<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    alignments: &[Alignment],
) -> AnyView {
    let mut header_rows = Vec::new();
    let mut body_rows = Vec::new();

    while let Some(event) = events.next() {
        match event {
            Event::Start(Tag::TableHead) => {
                header_rows.push(parse_table_cells(events, alignments, TagEnd::TableHead));
            }
            Event::Start(Tag::TableRow) => {
                body_rows.push(parse_table_cells(events, alignments, TagEnd::TableRow));
            }
            Event::End(TagEnd::Table) => break,
            _ => {}
        }
    }

    table([table_head(header_rows), table_body(body_rows)]).into_view()
}

/// Parses CommonMark table cells into one semantic header or body row.
///
/// # Arguments
///
/// * `events` — CommonMark event stream positioned inside a table row.
/// * `alignments` — Parsed alignment for each source column.
/// * `end` — Closing tag that terminates the header or body row.
///
/// # Returns
///
/// A semantic table-row view containing aligned cells.
fn parse_table_cells<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    alignments: &[Alignment],
    end: TagEnd,
) -> AnyView {
    let mut cells = Vec::new();

    while let Some(event) = events.next() {
        match event {
            Event::Start(Tag::TableCell) => {
                let alignment = alignment_at(alignments, cells.len());
                cells
                    .push(table_cell(parse_inline(events, TagEnd::TableCell)).alignment(alignment));
            }
            Event::End(tag) if tag == end => break,
            _ => {}
        }
    }

    table_row(cells).into_view()
}

/// Returns the semantic alignment for one parsed table column.
///
/// Missing and unspecified alignments use the semantic cell's left default.
///
/// # Arguments
///
/// * `alignments` — Parsed table-column alignments.
/// * `column` — Zero-based source column index.
///
/// # Returns
///
/// A [`CellAlignment`] for the requested table cell.
fn alignment_at(alignments: &[Alignment], column: usize) -> CellAlignment {
    match alignments.get(column).copied().unwrap_or(Alignment::None) {
        Alignment::None | Alignment::Left => CellAlignment::Left,
        Alignment::Center => CellAlignment::Center,
        Alignment::Right => CellAlignment::Right,
    }
}
