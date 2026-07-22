//! Semantic table collection, measurement, and rendering.
//!
//! This module resolves nested table styles, allocates responsive columns,
//! wraps rich cell content, and renders bordered table grids.

use ratatui::{
    layout::{Alignment, Rect},
    text::{Line, Text},
    widgets::{Block, Paragraph},
};

use crate::{
    app::Result,
    component::RenderCtx,
    style::{Color, TuiStyle},
    view::{
        metadata::StyleMetadata,
        model::{CellAlignment, View},
    },
};

use super::{VerticalSpan, line_count_height, resolve_style, resolved_rich_text, wrap_styled_line};

/// Render-ready table cell with its resolved text style.
#[derive(Clone)]
struct RenderedTableCell {
    /// Rich text rendered inside the cell.
    content: Text<'static>,
    /// Horizontal alignment applied after wrapping.
    alignment: CellAlignment,
    /// Fully resolved style for the cell text.
    style: TuiStyle,
    /// Whether a focused inline link requested scrolling into view.
    link_scroll_requested: bool,
}

/// Render-ready table row containing source-order cells.
struct RenderedTableRow {
    /// Cells present in the source row before normalization.
    cells: Vec<RenderedTableCell>,
    /// Section or row background painted beneath the row cells.
    background: Option<Color>,
}

/// Resolved semantic table data shared by rendering and measurement.
struct ResolvedTableLayout {
    /// Fully resolved style for the table container and borders.
    style: TuiStyle,
    /// Render-ready rows collected from the table sections.
    rows: Vec<RenderedTableRow>,
    /// Responsive content widths for visible leading columns.
    widths: Vec<u16>,
}

/// Collects semantic table rows and resolves nested section, row, and cell styles.
///
/// # Arguments
///
/// * `sections` — Table sections to traverse in source order.
/// * `ctx` — Rendering context carrying the table's inherited style.
///
/// # Returns
///
/// A [`Vec`] containing render-ready rows from valid table sections.
fn collect_table_rows(sections: &[View], ctx: &mut RenderCtx<'_, '_>) -> Vec<RenderedTableRow> {
    let mut rendered_rows = Vec::new();

    for section in sections {
        let (rows, metadata) = match section {
            View::TableHead { rows, metadata } | View::TableBody { rows, metadata } => {
                (rows, metadata)
            }
            _ => continue,
        };
        let section_style = resolve_style(metadata, ctx);
        let section_background = section_style.background;
        let area = ctx.area();
        let section_rows = ctx.with_area_inherited_style_and_selector_ancestor(
            area,
            section_style.inherited_values(),
            metadata.clone(),
            |ctx| {
                rows.iter()
                    .filter_map(|row| collect_table_row(row, section_background, ctx))
                    .collect::<Vec<_>>()
            },
        );
        rendered_rows.extend(section_rows);
    }

    rendered_rows
}

/// Collects one semantic table row and resolves its cell styles.
///
/// # Arguments
///
/// * `row` — Candidate table-row view.
/// * `section_background` — Optional section surface color beneath the row.
/// * `ctx` — Rendering context carrying the section's inherited style.
///
/// # Returns
///
/// An [`Option`] containing a render-ready row when `row` is a table row.
fn collect_table_row(
    row: &View,
    section_background: Option<Color>,
    ctx: &mut RenderCtx<'_, '_>,
) -> Option<RenderedTableRow> {
    let View::TableRow { cells, metadata } = row else {
        return None;
    };
    let row_style = resolve_style(metadata, ctx);
    let background = row_style.background.or(section_background);
    let area = ctx.area();
    let cells = ctx.with_area_inherited_style_and_selector_ancestor(
        area,
        row_style.inherited_values(),
        metadata.clone(),
        |ctx| {
            cells
                .iter()
                .map(|cell| collect_table_cell(cell, ctx))
                .collect()
        },
    );

    Some(RenderedTableRow { cells, background })
}

/// Collects a semantic table cell or an empty placeholder for an invalid child.
///
/// # Arguments
///
/// * `cell` — Candidate table-cell view.
/// * `ctx` — Rendering context carrying the row's inherited style.
///
/// # Returns
///
/// A [`RenderedTableCell`] preserving the source column position.
fn collect_table_cell(cell: &View, ctx: &mut RenderCtx<'_, '_>) -> RenderedTableCell {
    if let View::TableCell {
        content,
        alignment,
        metadata,
    } = cell
    {
        let style = resolve_style(metadata, ctx);
        return RenderedTableCell {
            content: resolved_rich_text(content, metadata, style, ctx),
            alignment: *alignment,
            style,
            link_scroll_requested: content.focused_link_requested_scroll(),
        };
    }

    RenderedTableCell {
        content: Text::default(),
        alignment: CellAlignment::Left,
        style: ctx.inherited_style(),
        link_scroll_requested: false,
    }
}

/// Resolves semantic table rows, styles, and responsive column widths.
///
/// # Arguments
///
/// * `sections` — Header and body sections to collect.
/// * `metadata` — Selector metadata for the table container.
/// * `ctx` — Rendering context containing inherited styles and viewport width.
///
/// # Returns
///
/// A [`ResolvedTableLayout`] shared by rendering and intrinsic measurement.
fn resolve_table_layout(
    sections: &[View],
    metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> ResolvedTableLayout {
    let style = resolve_style(metadata, ctx);
    let area = ctx.area();
    let rows = ctx.with_area_inherited_style_and_selector_ancestor(
        area,
        style.inherited_values(),
        metadata.clone(),
        |ctx| collect_table_rows(sections, ctx),
    );
    let widths = table_column_widths(&rows, area.width);

    ResolvedTableLayout {
        style,
        rows,
        widths,
    }
}

/// Allocates visible table-column widths within the viewport budget.
///
/// Columns begin at their widest logical content line. When those preferred
/// widths exceed the viewport, the widest columns are capped evenly without
/// reducing any visible column below one content cell. Trailing columns that
/// cannot receive a content cell plus their border are omitted.
///
/// # Arguments
///
/// * `rows` — Render-ready table rows used for preferred-width measurement.
/// * `available_width` — Total viewport width including table borders.
///
/// # Returns
///
/// A [`Vec`] containing one content width for each visible leading column.
fn table_column_widths(rows: &[RenderedTableRow], available_width: u16) -> Vec<u16> {
    let column_count = rows.iter().map(|row| row.cells.len()).max().unwrap_or(0);
    let visible_count = column_count.min(usize::from(available_width.saturating_sub(1) / 2));
    if visible_count == 0 {
        return Vec::new();
    }

    let preferred = (0..visible_count)
        .map(|column| {
            rows.iter()
                .filter_map(|row| row.cells.get(column))
                .map(|cell| u16::try_from(cell.content.width()).unwrap_or(u16::MAX))
                .max()
                .unwrap_or(1)
                .max(1)
        })
        .collect::<Vec<_>>();
    let border_width = u16::try_from(visible_count.saturating_add(1)).unwrap_or(u16::MAX);
    let content_budget = available_width.saturating_sub(border_width);
    let preferred_total = preferred.iter().copied().map(u32::from).sum::<u32>();
    if preferred_total <= u32::from(content_budget) {
        return preferred;
    }

    let max_preferred = preferred.iter().copied().max().unwrap_or(1);
    let mut low = 1u16;
    let mut high = max_preferred;
    while low < high {
        let midpoint = low + (high - low).div_ceil(2);
        let total = preferred
            .iter()
            .map(|width| u32::from((*width).min(midpoint)))
            .sum::<u32>();
        if total <= u32::from(content_budget) {
            low = midpoint;
        } else {
            high = midpoint.saturating_sub(1);
        }
    }

    let mut widths = preferred
        .iter()
        .map(|width| (*width).min(low))
        .collect::<Vec<_>>();
    let used = widths.iter().copied().map(u32::from).sum::<u32>();
    let mut remaining = u32::from(content_budget).saturating_sub(used);
    for (width, preferred_width) in widths.iter_mut().zip(preferred.iter()) {
        if remaining == 0 {
            break;
        }
        if *width < *preferred_width {
            *width = width.saturating_add(1);
            remaining -= 1;
        }
    }

    widths
}

/// Returns the wrapped height of a normalized table row.
///
/// # Arguments
///
/// * `row` — Row whose existing cells should be measured.
/// * `widths` — Allocated widths for visible normalized columns.
///
/// # Returns
///
/// A [`u16`] height of at least one terminal row.
fn table_row_height(row: &RenderedTableRow, widths: &[u16]) -> u16 {
    widths
        .iter()
        .enumerate()
        .filter_map(|(column, width)| {
            row.cells
                .get(column)
                .map(|cell| line_count_height(wrapped_table_cell_text(cell, *width).lines.len()))
        })
        .max()
        .unwrap_or(1)
        .max(1)
}

/// Wraps table-cell rich text without splitting double-width graphemes.
///
/// Ratatui's general paragraph wrapper can place an indivisible wide grapheme
/// across a narrow area boundary. Table cells pre-wrap at grapheme boundaries
/// so content never overwrites the following column separator.
///
/// # Arguments
///
/// * `cell` — Render-ready cell containing rich text and its resolved style.
/// * `width` — Allocated terminal-cell width for the column.
///
/// # Returns
///
/// A [`Text`] value whose logical lines all fit within `width`.
fn wrapped_table_cell_text(cell: &RenderedTableCell, width: u16) -> Text<'static> {
    if width == 0 {
        return Text::default();
    }

    let base_style = cell.style.to_ratatui_style();
    let mut wrapped = cell
        .content
        .lines
        .iter()
        .flat_map(|line| wrap_styled_line(line, width, base_style))
        .collect::<Vec<_>>();

    if wrapped.is_empty() {
        wrapped.push(Line::default());
    }

    Text::from(wrapped)
}

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
fn table_border_line(widths: &[u16], position: TableBorderPosition) -> String {
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
fn table_vertical_border(height: u16) -> Text<'static> {
    Text::from(vec![Line::raw("│"); usize::from(height)])
}

/// Position of a horizontal table border within the rendered grid.
#[derive(Clone, Copy)]
enum TableBorderPosition {
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
fn ratatui_cell_alignment(alignment: CellAlignment) -> Alignment {
    match alignment {
        CellAlignment::Left => Alignment::Left,
        CellAlignment::Center => Alignment::Center,
        CellAlignment::Right => Alignment::Right,
    }
}

/// Renders a semantic table with responsive columns and variable-height rows.
///
/// # Arguments
///
/// * `sections` — Header and body sections to render.
/// * `metadata` — Selector metadata for the table container.
/// * `ctx` — Rendering context containing the available viewport.
///
/// # Returns
///
/// An empty [`Result`] on success.
pub(super) fn render_table_view(
    sections: &[View],
    metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> Result<()> {
    let ResolvedTableLayout {
        style: table_style,
        rows,
        widths,
    } = resolve_table_layout(sections, metadata, ctx);
    let area = ctx.area();
    if widths.is_empty() || rows.is_empty() || area.height == 0 {
        return Ok(());
    }

    let table_width = widths
        .iter()
        .copied()
        .fold(
            u16::try_from(widths.len().saturating_add(1)).unwrap_or(u16::MAX),
            u16::saturating_add,
        )
        .min(area.width);
    let table_area = Rect {
        width: table_width,
        ..area
    };
    let source_rows = table_source_rows(sections);
    let border_style = table_style.to_ratatui_style();
    ctx.with_area(table_area, |ctx| {
        ctx.render_widget(Block::new().style(border_style));
    });
    let mut y = table_area.y;
    let bottom = table_area.y.saturating_add(table_area.height);
    ctx.with_area(
        Rect {
            height: 1,
            ..table_area
        },
        |ctx| {
            ctx.render_widget(
                Paragraph::new(table_border_line(&widths, TableBorderPosition::Top))
                    .style(border_style),
            );
        },
    );
    y = y.saturating_add(1);

    for (row_index, row) in rows.iter().enumerate() {
        if y >= bottom {
            break;
        }
        let requested_height = table_row_height(row, &widths);
        let rendered_height = requested_height.min(bottom.saturating_sub(y));
        if let Some(background) = row.background {
            ctx.with_area(
                Rect {
                    y,
                    height: rendered_height,
                    ..table_area
                },
                |ctx| {
                    ctx.render_widget(
                        Block::new().style(ratatui::style::Style::new().bg(background)),
                    );
                },
            );
        }
        let mut x = table_area.x;
        ctx.with_area(
            Rect {
                x,
                y,
                width: 1,
                height: rendered_height,
            },
            |ctx| {
                ctx.render_widget(
                    Paragraph::new(table_vertical_border(rendered_height)).style(border_style),
                );
            },
        );
        x = x.saturating_add(1);

        for (column, width) in widths.iter().copied().enumerate() {
            if let Some(cell) = row.cells.get(column) {
                let cell_area = Rect {
                    x,
                    y,
                    width,
                    height: rendered_height,
                };
                ctx.with_area(cell_area, |ctx| {
                    ctx.render_widget(
                        Paragraph::new(wrapped_table_cell_text(cell, width))
                            .style(cell.style.to_ratatui_style())
                            .alignment(ratatui_cell_alignment(cell.alignment)),
                    );
                });
                if let Some(View::TableCell {
                    content, alignment, ..
                }) = source_rows
                    .get(row_index)
                    .and_then(|source_cells| source_cells.get(column))
                {
                    content.record_link_hit_areas(cell_area, width, *alignment, ctx);
                }
            }
            x = x.saturating_add(width);
            ctx.with_area(
                Rect {
                    x,
                    y,
                    width: 1,
                    height: rendered_height,
                },
                |ctx| {
                    ctx.render_widget(
                        Paragraph::new(table_vertical_border(rendered_height)).style(border_style),
                    );
                },
            );
            x = x.saturating_add(1);
        }
        y = y.saturating_add(rendered_height);
        if rendered_height < requested_height || y >= bottom {
            break;
        }

        let position = if row_index + 1 == rows.len() {
            TableBorderPosition::Bottom
        } else {
            TableBorderPosition::Middle
        };
        ctx.with_area(
            Rect {
                y,
                height: 1,
                ..table_area
            },
            |ctx| {
                ctx.render_widget(
                    Paragraph::new(table_border_line(&widths, position)).style(border_style),
                );
            },
        );
        y = y.saturating_add(1);
    }

    clear_table_link_scroll_requests(sections);

    Ok(())
}

/// Returns source table rows in the same order used by resolved rendering.
fn table_source_rows(sections: &[View]) -> Vec<&[View]> {
    sections
        .iter()
        .filter_map(|section| match section {
            View::TableHead { rows, .. } | View::TableBody { rows, .. } => Some(rows),
            _ => None,
        })
        .flat_map(|rows| rows.iter())
        .filter_map(|row| match row {
            View::TableRow { cells, .. } => Some(cells.as_slice()),
            _ => None,
        })
        .collect()
}

/// Clears completed inline-link scroll requests throughout a semantic table.
///
/// # Arguments
///
/// * `views` — Table sections, rows, or cells to traverse.
fn clear_table_link_scroll_requests(views: &[View]) {
    for view in views {
        match view {
            View::TableHead { rows: children, .. }
            | View::TableBody { rows: children, .. }
            | View::TableRow {
                cells: children, ..
            } => clear_table_link_scroll_requests(children),
            View::TableCell { content, .. } => content.clear_link_scroll_requests(),
            _ => {}
        }
    }
}

/// Returns the intrinsic height of a semantic table.
///
/// # Arguments
///
/// * `sections` — Header and body sections to measure.
/// * `metadata` — Selector metadata for the table container.
/// * `ctx` — Rendering context containing the available width.
///
/// # Returns
///
/// A [`u16`] height including horizontal row boundaries.
pub(super) fn min_height_for_table_view(
    sections: &[View],
    metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> u16 {
    let ResolvedTableLayout { rows, widths, .. } = resolve_table_layout(sections, metadata, ctx);
    if widths.is_empty() || rows.is_empty() {
        return 0;
    }

    rows.iter().map(|row| table_row_height(row, &widths)).fold(
        u16::try_from(rows.len().saturating_add(1)).unwrap_or(u16::MAX),
        u16::saturating_add,
    )
}

/// Returns the focused linked table row's vertical span.
///
/// # Arguments
///
/// * `sections` — Header and body sections to inspect.
/// * `metadata` — Selector metadata for the table container.
/// * `ctx` — Rendering context containing the available width.
///
/// # Returns
///
/// An [`Option`] containing the focused row span between table borders.
pub(super) fn focused_link_span_for_table_view(
    sections: &[View],
    metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> Option<VerticalSpan> {
    let ResolvedTableLayout { rows, widths, .. } = resolve_table_layout(sections, metadata, ctx);
    if rows.is_empty() || widths.is_empty() {
        return None;
    }

    let mut top = 1u32;
    for row in &rows {
        let height = u32::from(table_row_height(row, &widths));
        if row.cells.iter().any(|cell| cell.link_scroll_requested) {
            return Some(VerticalSpan {
                top,
                bottom: top.saturating_add(height),
            });
        }
        top = top.saturating_add(height).saturating_add(1);
    }

    None
}
