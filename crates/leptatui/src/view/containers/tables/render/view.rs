//! Semantic table drawing and intrinsic height calculation.

use ratatui::{
    layout::Rect,
    widgets::{Block, Paragraph},
};

use super::{
    grid::{TableBorderPosition, ratatui_cell_alignment, table_border_line, table_vertical_border},
    layout::{
        ResolvedTableLayout, resolve_table_layout, table_row_height, wrapped_table_cell_text,
    },
};
use crate::{
    app::Result,
    component::RenderCtx,
    view::{
        AnyView, StyleMetadata, TableCellView, TableRowView, TableSectionView,
        core::render::VerticalSpan, link::RichTextWrapMode,
    },
};

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
///
/// # Errors
///
/// This implementation currently renders only infallible widgets and does not
/// produce an error.
pub(in crate::view::containers::tables) fn render_table_view(
    sections: &[AnyView],
    metadata: &StyleMetadata,
    ctx: &mut RenderCtx<'_, '_>,
) -> Result<()> {
    let ResolvedTableLayout {
        style: table_style,
        rows,
        widths,
    } = resolve_table_layout(sections, metadata, ctx);
    let geometry = ctx.active_layout_geometry(metadata);
    if let Some(geometry) = geometry {
        ctx.with_area(geometry.border_box, |ctx| {
            ctx.render_widget(table_style.to_block());
        });
    }
    let area = geometry.map_or_else(|| ctx.area(), |geometry| geometry.content_box);
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
                if let Some(source_cell) = source_rows
                    .get(row_index)
                    .and_then(|source_cells| source_cells.get(column))
                    .and_then(|cell| cell.downcast_ref::<TableCellView>())
                {
                    source_cell.content.record_link_hit_areas(
                        cell_area,
                        width,
                        source_cell.alignment,
                        RichTextWrapMode::Grapheme,
                        ctx,
                    );
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
///
/// Non-section and non-row views are omitted to match layout resolution.
///
/// # Arguments
///
/// * `sections` — Semantic table sections containing source rows and cells.
///
/// # Returns
///
/// A [`Vec`] of borrowed source-cell slices in rendered row order.
fn table_source_rows(sections: &[AnyView]) -> Vec<&[AnyView]> {
    sections
        .iter()
        .filter_map(|section| section.downcast_ref::<TableSectionView>())
        .flat_map(|section| section.children.iter())
        .filter_map(|row| {
            row.downcast_ref::<TableRowView>()
                .map(|row| row.children.as_slice())
        })
        .collect()
}

/// Clears completed inline-link scroll requests throughout a semantic table.
///
/// # Arguments
///
/// * `sections` — Semantic table sections whose source cells should be reset.
fn clear_table_link_scroll_requests(sections: &[AnyView]) {
    for cells in table_source_rows(sections) {
        for cell in cells {
            if let Some(cell) = cell.downcast_ref::<TableCellView>() {
                cell.content.clear_link_scroll_requests();
            }
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
pub(in crate::view::containers::tables) fn intrinsic_height_for_table_view(
    sections: &[AnyView],
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
/// The returned coordinates include the table's top border and inter-row
/// borders used by resolved rendering.
///
/// # Arguments
///
/// * `sections` — Semantic table sections to resolve and inspect.
/// * `metadata` — Table metadata used during layout resolution.
/// * `ctx` — Render context supplying the available area and stylesheets.
///
/// # Returns
///
/// An [`Option`] containing the rendered row span with a pending focused-link
/// scroll request.
pub(in crate::view::containers::tables) fn focused_link_span_for_table_view(
    sections: &[AnyView],
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
