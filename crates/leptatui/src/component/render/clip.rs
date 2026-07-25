//! Offscreen clipped view rendering and cursor remapping.

use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
};

use crate::{
    StyleMetadata, app::Result, style::TuiStyle, terminal_image::TerminalImageSupport,
    view::AnyView,
};

use super::{RenderCtx, target::RenderTarget};

impl RenderCtx<'_, '_> {
    /// Renders a view into an offscreen buffer and copies a clipped rectangle.
    ///
    /// # Arguments
    ///
    /// * `view` — View rendered into the offscreen buffer.
    /// * `geometry` — Full child geometry expressed in offscreen coordinates.
    /// * `source` — First offscreen column and row copied into the target.
    /// * `target_area` — Visible destination area receiving copied cells.
    /// * `inherited_style` — Style values inherited by the clipped view.
    /// * `selector_ancestor` — Parent metadata appended to selector ancestry.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    ///
    /// # Errors
    ///
    /// Returns [`crate::app::Error::Io`] if rendering performs terminal I/O
    /// that fails.
    pub(crate) fn render_view_clipped(
        &mut self,
        view: &AnyView,
        geometry: crate::LayoutGeometry,
        source: Position,
        target_area: Rect,
        inherited_style: TuiStyle,
        selector_ancestor: StyleMetadata,
    ) -> Result<()> {
        let full_area = geometry.border_box;
        if target_area.width == 0 || target_area.height == 0 || full_area.height == 0 {
            return Ok(());
        }

        if self.target.supports_terminal_images() && self.terminal_images.supports_protocol() {
            let handled = self.with_assigned_layout_geometry_and_selector_ancestor(
                geometry,
                view.style_metadata(),
                inherited_style,
                selector_ancestor.clone(),
                |ctx| view.render_terminal_image_clipped(source.x, source.y, target_area, ctx),
            )?;
            if handled {
                return Ok(());
            }
        }

        let mut buffer = Buffer::empty(Rect::new(0, 0, full_area.width, full_area.height));
        {
            let target = self.target.buffer_mut();
            for y in 0..target_area.height {
                for x in 0..target_area.width {
                    let target_position = (
                        target_area.x.saturating_add(x),
                        target_area.y.saturating_add(y),
                    );
                    let buffer_position = (source.x.saturating_add(x), source.y.saturating_add(y));

                    if let (Some(target_cell), Some(buffer_cell)) = (
                        target.cell(target_position),
                        buffer.cell_mut(buffer_position),
                    ) {
                        *buffer_cell = target_cell.clone();
                    }
                }
            }
        }

        let mut selector_ancestors = self.selector_ancestors.clone();
        selector_ancestors.push(selector_ancestor);

        let mut cursor_position = None;

        {
            let mut buffer_ctx = RenderCtx {
                target: RenderTarget::Buffer {
                    buffer: &mut buffer,
                    cursor_position: &mut cursor_position,
                },
                area: Rect::new(0, 0, full_area.width, full_area.height),
                geometry,
                geometry_owner: view.style_metadata().map(std::ptr::from_ref),
                viewport_size: self.viewport_size,
                stylesheets: self.stylesheets.clone(),
                inherited_style,
                selector_ancestors,
                terminal_images: TerminalImageSupport::default(),
                hit_mapper: self.hit_mapper.with_clipped_child(
                    Rect {
                        x: source.x,
                        y: source.y,
                        width: target_area.width,
                        height: target_area.height,
                    },
                    target_area,
                ),
                layout_state: self.layout_state.for_assigned_area(),
            };
            view.as_view().render(&mut buffer_ctx)?;
        }

        let target = self.target.buffer_mut();
        for y in 0..target_area.height {
            for x in 0..target_area.width {
                let source_cell =
                    buffer[(source.x.saturating_add(x), source.y.saturating_add(y))].clone();
                let destination_position = (
                    target_area.x.saturating_add(x),
                    target_area.y.saturating_add(y),
                );
                if let Some(destination) = target.cell_mut(destination_position) {
                    *destination = source_cell;
                }
            }
        }

        if let Some(position) = cursor_position
            && position.y >= source.y
            && position.y < source.y.saturating_add(target_area.height)
            && position.x >= source.x
            && position.x < source.x.saturating_add(target_area.width)
        {
            self.set_cursor_position(Position {
                x: target_area
                    .x
                    .saturating_add(position.x.saturating_sub(source.x)),
                y: target_area
                    .y
                    .saturating_add(position.y.saturating_sub(source.y)),
            });
        }

        Ok(())
    }
}
