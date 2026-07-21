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
    view::{AnyView, StyleMetadata},
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

    Ok(())
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
pub(in crate::view::containers::tables) fn min_height_for_table_view(
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
