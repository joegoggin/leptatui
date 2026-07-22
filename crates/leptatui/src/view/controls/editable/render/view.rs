//! Editable-control rendering and intrinsic height measurement.

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
    Borders, RenderCtx, Result,
    view::core::render::{
        line_count_height, resolve_style, vertical_border_rows, vertical_padding_rows,
    },
};

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
    let style = resolve_style(&view.metadata, ctx);
    ctx.record_metadata_hit_area(&view.metadata);
    let block = style.to_block_with_default_borders(Borders::ALL);
    let inner = block.inner(ctx.area());
    let pending = pending_insert_render(&view.value, &view.editable_state);
    let display_value = if let Some(pending) = pending.as_ref() {
        pending.value.as_str()
    } else if view.value.is_empty() {
        view.placeholder.as_deref().unwrap_or("")
    } else {
        view.value.as_str()
    };
    ctx.render_widget(block);

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

/// Returns the intrinsic height of a controlled input or text area.
///
/// # Arguments
///
/// * `view` — Editable control model to measure.
/// * `ctx` — Rendering context containing the available width and styles.
///
/// # Returns
///
/// A [`u16`] intrinsic height including borders and padding.
pub(crate) fn min_height_for_editable_text_view(
    view: &EditableModel,
    ctx: &mut RenderCtx<'_, '_>,
) -> u16 {
    let style = resolve_style(&view.metadata, ctx);
    let border_height = vertical_border_rows(style.borders.unwrap_or(Borders::ALL));
    let padding_height = vertical_padding_rows(style.padding);
    if view.kind == EditableControlKind::Input {
        return 1u16
            .saturating_add(border_height)
            .saturating_add(padding_height);
    }

    let display_value = if view.value.is_empty() {
        view.placeholder.as_deref().unwrap_or("")
    } else {
        &view.value
    };
    let inner = style
        .to_block_with_default_borders(Borders::ALL)
        .inner(ctx.area());
    line_count_height(
        text_area_paragraph(display_value, style, 0, 0, None)
            .line_count(inner.width)
            .saturating_add(trailing_text_area_empty_line_rows(display_value)),
    )
    .max(1)
    .saturating_add(border_height)
    .saturating_add(padding_height)
}
