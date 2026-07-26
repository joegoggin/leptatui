//! Semantic collection, responsive sizing, and rich cell wrapping.

use ratatui::text::{Line, Text};

use crate::view::content::code_block::wrap_styled_line;
use crate::view::core::render::{line_count_height, resolve_style};
use crate::view::link::resolved_rich_text;
use crate::{
    component::RenderCtx,
    style::{Color, TuiStyle},
    view::{AnyView, CellAlignment, StyleMetadata, TableCellView, TableRowView, TableSectionView},
};

/// Render-ready table cell with its resolved text style.
#[derive(Clone)]
pub(super) struct RenderedTableCell {
    /// Rich text rendered inside the cell.
    pub(super) content: Text<'static>,
    /// Horizontal alignment applied after wrapping.
    pub(super) alignment: CellAlignment,
    /// Fully resolved style for the cell text.
    pub(super) style: TuiStyle,
    /// Whether a focused inline link requested scrolling into view.
    pub(super) link_scroll_requested: bool,
}

/// Render-ready table row containing source-order cells.
pub(super) struct RenderedTableRow {
    /// Cells present in the source row before normalization.
    pub(super) cells: Vec<RenderedTableCell>,
    /// Section or row background painted beneath the row cells.
    pub(super) background: Option<Color>,
}

/// Resolved semantic table data shared by rendering and measurement.
pub(super) struct ResolvedTableLayout {
    /// Fully resolved style for the table container and borders.
    pub(super) style: TuiStyle,
    /// Render-ready rows collected from the table sections.
    pub(super) rows: Vec<RenderedTableRow>,
    /// Responsive content widths for visible leading columns.
    pub(super) widths: Vec<u16>,
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
fn collect_table_rows(sections: &[AnyView], ctx: &mut RenderCtx<'_, '_>) -> Vec<RenderedTableRow> {
    let mut rendered_rows = Vec::new();

    for section in sections {
        let Some(section) = section.downcast_ref::<TableSectionView>() else {
            continue;
        };
        let rows = &section.children;
        let metadata = &section.metadata;
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
    row: &AnyView,
    section_background: Option<Color>,
    ctx: &mut RenderCtx<'_, '_>,
) -> Option<RenderedTableRow> {
    let row = row.downcast_ref::<TableRowView>()?;
    let cells = &row.children;
    let metadata = &row.metadata;
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
fn collect_table_cell(cell: &AnyView, ctx: &mut RenderCtx<'_, '_>) -> RenderedTableCell {
    if let Some(cell) = cell.downcast_ref::<TableCellView>() {
        let style = resolve_style(&cell.metadata, ctx);
        return RenderedTableCell {
            content: resolved_rich_text(&cell.content, &cell.metadata, &style, ctx),
            alignment: cell.alignment,
            style,
            link_scroll_requested: cell.content.focused_link_requested_scroll(),
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
pub(super) fn resolve_table_layout(
    sections: &[AnyView],
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
pub(super) fn table_row_height(row: &RenderedTableRow, widths: &[u16]) -> u16 {
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
pub(super) fn wrapped_table_cell_text(cell: &RenderedTableCell, width: u16) -> Text<'static> {
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
