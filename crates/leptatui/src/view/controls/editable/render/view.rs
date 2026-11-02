//! Editable-control rendering and intrinsic two-axis measurement.

use super::super::{
    model::{EditableControlKind, EditableModel},
    visual::visual_selection_range,
};
use super::{
    content::{
        input_paragraph, pending_insert_render, text_area_paragraph,
        trailing_text_area_empty_line_rows,
    },
    geometry::{
        input_horizontal_scroll, set_input_cursor, set_text_area_cursor,
        set_text_area_pending_insert_cursor, text_area_vertical_scroll,
    },
};
use crate::{
    Borders, LayoutSize, RenderCtx, Result,
    view::core::{
        measurement::{AvailableSpace, measure_fixed, measure_rich_text, sanitize_cells},
        render::{
            horizontal_border_columns, horizontal_padding_columns, resolve_style,
            vertical_border_rows, vertical_padding_rows,
        },
    },
};
use ratatui::text::Text;
use unicode_width::UnicodeWidthStr;

/// Renders a controlled input or text area.
///
/// # Arguments
///
/// * `view` — Editable control model to render.
/// * `ctx` — Active rendering context containing the target area and styles.
///
/// # Returns
///
/// An empty [`Result`] on success.
///
/// # Errors
///
/// This implementation currently renders only infallible widgets and does not
/// produce an error.
pub(crate) fn render_editable_text_view(
    view: &EditableModel,
    ctx: &mut RenderCtx<'_, '_>,
) -> Result<()> {
    view.sync_style_state();
    let style = resolve_style(&view.metadata, ctx);
    ctx.record_metadata_hit_area(&view.metadata);
    let block = style.to_block_with_default_borders(Borders::ALL);
    let geometry = ctx.active_layout_geometry(&view.metadata);
    let inner = geometry.map_or_else(|| block.inner(ctx.area()), |geometry| geometry.content_box);
    let pending = pending_insert_render(&view.value, &view.editable_state);
    let display_value = if let Some(pending) = pending.as_ref() {
        pending.value.as_str()
    } else if view.value.is_empty() {
        view.placeholder.as_deref().unwrap_or("")
    } else {
        view.value.as_str()
    };
    if let Some(geometry) = geometry {
        ctx.with_area(geometry.border_box, |ctx| {
            ctx.render_widget(block);
        });
    } else {
        ctx.render_widget(block);
    }

    match view.kind {
        EditableControlKind::Input => {
            let horizontal_scroll = if let Some(pending) = pending.as_ref() {
                input_horizontal_scroll(
                    &pending.value,
                    pending.scroll_cursor,
                    view.editable_state.horizontal_scroll(),
                    inner.width,
                )
            } else if view.value.is_empty() {
                0
            } else {
                input_horizontal_scroll(
                    &view.value,
                    view.editable_state.cursor(),
                    view.editable_state.horizontal_scroll(),
                    inner.width,
                )
            };
            ctx.with_area(inner, |ctx| {
                ctx.render_widget(input_paragraph(
                    display_value,
                    style,
                    horizontal_scroll,
                    pending
                        .as_ref()
                        .and_then(|pending| pending.selection.clone())
                        .or_else(|| {
                            visual_selection_range(
                                &view.value,
                                &view.editable_state,
                                EditableControlKind::Input,
                            )
                        }),
                ));
                if view.metadata.is_focused() {
                    let (value, cursor) = pending.as_ref().map_or(
                        (view.value.as_str(), view.editable_state.cursor()),
                        |pending| (pending.value.as_str(), pending.cursor),
                    );
                    set_input_cursor(value, cursor, horizontal_scroll, ctx);
                }
            });
        }
        EditableControlKind::TextArea => {
            let vertical_scroll = if let Some(pending) = pending.as_ref() {
                text_area_vertical_scroll(
                    &pending.value,
                    pending.scroll_cursor,
                    view.editable_state.vertical_scroll(),
                    inner.height,
                    inner.width,
                )
            } else if view.value.is_empty() {
                0
            } else {
                text_area_vertical_scroll(
                    &view.value,
                    view.editable_state.cursor(),
                    view.editable_state.vertical_scroll(),
                    inner.height,
                    inner.width,
                )
            };
            ctx.with_area(inner, |ctx| {
                ctx.render_widget(text_area_paragraph(
                    display_value,
                    style,
                    vertical_scroll,
                    view.editable_state.horizontal_scroll(),
                    pending
                        .as_ref()
                        .and_then(|pending| pending.selection.clone())
                        .or_else(|| {
                            visual_selection_range(
                                &view.value,
                                &view.editable_state,
                                EditableControlKind::TextArea,
                            )
                        }),
                ));
                if view.metadata.is_focused() {
                    if let Some(pending) = pending.as_ref() {
                        set_text_area_pending_insert_cursor(
                            &pending.value,
                            pending.cursor,
                            vertical_scroll,
                            view.editable_state.horizontal_scroll(),
                            ctx,
                        );
                    } else {
                        set_text_area_cursor(
                            &view.value,
                            view.editable_state.cursor(),
                            vertical_scroll,
                            view.editable_state.horizontal_scroll(),
                            ctx,
                        );
                    }
                }
            });
        }
    }
    view.metadata.clear_scroll_into_view_request();
    Ok(())
}

/// Returns the intrinsic size of a controlled input or text area.
///
/// # Arguments
///
/// * `view` — Editable control model to measure.
/// * `known_dimensions` — Exact dimensions supplied by parent layout.
/// * `available_space` — Soft constraints for unknown dimensions.
/// * `ctx` — Rendering context containing resolved styles.
///
/// # Returns
///
/// A [`LayoutSize`] including the control's borders and padding.
pub(crate) fn measure_editable_text_view(
    view: &EditableModel,
    known_dimensions: LayoutSize<Option<f32>>,
    available_space: LayoutSize<AvailableSpace>,
    ctx: &mut RenderCtx<'_, '_>,
) -> LayoutSize<f32> {
    view.sync_style_state();
    let style = resolve_style(&view.metadata, ctx);
    let borders = style.borders.unwrap_or(Borders::ALL);
    let border_width = horizontal_border_columns(borders);
    let padding_width = horizontal_padding_columns(style.padding);
    let horizontal_inset = border_width.saturating_add(padding_width);
    let border_height = vertical_border_rows(borders);
    let padding_height = vertical_padding_rows(style.padding);
    let vertical_inset = border_height.saturating_add(padding_height);
    let display_value = if view.value.is_empty() {
        view.placeholder.as_deref().unwrap_or("")
    } else {
        &view.value
    };

    if view.kind == EditableControlKind::Input {
        let content_width = u16::try_from(UnicodeWidthStr::width(display_value))
            .unwrap_or(u16::MAX)
            .max(1);
        return measure_fixed(
            LayoutSize::new(
                f32::from(content_width.saturating_add(horizontal_inset)),
                f32::from(1u16.saturating_add(vertical_inset)),
            ),
            known_dimensions,
        );
    }

    let inset_width = f32::from(horizontal_inset);
    let inset_height = f32::from(vertical_inset);
    let content_known = LayoutSize::new(
        known_dimensions
            .width
            .map(|width| sanitize_cells(sanitize_cells(width) - inset_width)),
        known_dimensions
            .height
            .map(|height| sanitize_cells(sanitize_cells(height) - inset_height)),
    );
    let content_available = LayoutSize::new(
        match available_space.width {
            AvailableSpace::Definite(width) => {
                AvailableSpace::Definite(sanitize_cells(sanitize_cells(width) - inset_width))
            }
            constraint => constraint,
        },
        match available_space.height {
            AvailableSpace::Definite(height) => {
                AvailableSpace::Definite(sanitize_cells(sanitize_cells(height) - inset_height))
            }
            constraint => constraint,
        },
    );
    let text = Text::from(display_value.to_owned());
    let mut measured = measure_rich_text(&text, style, content_known, content_available);
    if known_dimensions.height.is_none() {
        let trailing_rows =
            u16::try_from(trailing_text_area_empty_line_rows(display_value)).unwrap_or(u16::MAX);
        measured.height = measured.height.max(1.0) + f32::from(trailing_rows);
    }

    LayoutSize::new(
        known_dimensions.width.map_or_else(
            || sanitize_cells(measured.width + inset_width),
            sanitize_cells,
        ),
        known_dimensions.height.map_or_else(
            || sanitize_cells(measured.height + inset_height),
            sanitize_cells,
        ),
    )
}
