//! Rendering, measurement, and image clipping across type erasure.

use ratatui::layout::{Rect, Size};

use super::AnyView;
use crate::{
    app::Result,
    component::RenderCtx,
    style::LayoutSize,
    view::{
        ImageView,
        core::{layout::render_with_layout, measurement::AvailableSpace, render::resolve_style},
        media::image::{ImageSource, image_render_area},
    },
};

impl AnyView {
    /// Renders the stored concrete node.
    ///
    /// Prior hit areas are cleared before the current root metadata and
    /// concrete node are rendered. Intermediate stacking-path traversal keeps
    /// hit areas recorded by the box's earlier shell paint.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Render context containing the target area and stylesheets.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] after the concrete node renders successfully.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Io`] if concrete rendering performs terminal
    /// I/O that fails.
    pub fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        render_with_layout(self.as_view(), ctx, |ctx| self.as_view().render(ctx))
    }

    /// Returns the intrinsic size of the stored node.
    ///
    /// # Arguments
    ///
    /// * `known_dimensions` — Exact dimensions supplied by parent layout.
    /// * `available_space` — Soft constraints for unknown dimensions.
    /// * `ctx` — Rendering context containing styles and inherited state.
    ///
    /// # Returns
    ///
    /// A [`LayoutSize`] containing measured terminal-cell width and height.
    pub fn measure(
        &self,
        known_dimensions: LayoutSize<Option<f32>>,
        available_space: LayoutSize<AvailableSpace>,
        ctx: &mut RenderCtx<'_, '_>,
    ) -> LayoutSize<f32> {
        if self.is_layout_hidden() {
            return LayoutSize::all(0.0);
        }
        self.as_view()
            .measure(known_dimensions, available_space, ctx)
    }

    /// Renders a clipped segment when the stored node is an image.
    ///
    /// # Arguments
    ///
    /// * `source_x` — First source column retained from the full view box.
    /// * `source_y` — First source row retained from the full view box.
    /// * `target_area` — Visible destination rectangle.
    /// * `ctx` — Render context carrying the full view geometry.
    ///
    /// # Returns
    ///
    /// A [`Result`] containing whether the stored node handled image rendering.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Io`] if fallback rendering performs terminal
    /// I/O that fails.
    pub(crate) fn render_terminal_image_clipped(
        &self,
        source_x: u16,
        source_y: u16,
        target_area: Rect,
        ctx: &mut RenderCtx<'_, '_>,
    ) -> Result<bool> {
        let Some(image) = self.downcast_ref::<ImageView>() else {
            return Ok(false);
        };

        let style = resolve_style(&image.metadata, ctx);
        let geometry = ctx.layout_geometry();
        let full_image_area = image_render_area(geometry.content_box, style.image_size);
        let source_right = source_x.saturating_add(target_area.width);
        let source_bottom = source_y.saturating_add(target_area.height);
        if source_x >= full_image_area.right()
            || source_y >= full_image_area.bottom()
            || source_right <= full_image_area.x
            || source_bottom <= full_image_area.y
        {
            return Ok(true);
        }

        let visible_source_x = source_x.max(full_image_area.x);
        let visible_source_y = source_y.max(full_image_area.y);
        let image_source_x = visible_source_x.saturating_sub(full_image_area.x);
        let image_source_y = visible_source_y.saturating_sub(full_image_area.y);
        let target_offset_x = visible_source_x.saturating_sub(source_x);
        let target_offset_y = visible_source_y.saturating_sub(source_y);
        let width = full_image_area
            .right()
            .saturating_sub(visible_source_x)
            .min(target_area.width.saturating_sub(target_offset_x));
        let height = full_image_area
            .bottom()
            .saturating_sub(visible_source_y)
            .min(target_area.height.saturating_sub(target_offset_y));
        if width == 0 || height == 0 {
            return Ok(true);
        }

        let ImageSource::Path(path) = &image.source;
        let render_area = Rect {
            x: target_area.x.saturating_add(target_offset_x),
            y: target_area.y.saturating_add(target_offset_y),
            width,
            height,
        };
        let full_size = Size::new(full_image_area.width, full_image_area.height);
        ctx.with_area(render_area, |ctx| {
            ctx.render_terminal_image_path_clipped(
                path,
                image.alt.as_deref(),
                style.to_ratatui_style(),
                full_size,
                image_source_x,
                image_source_y,
            );
        });
        Ok(true)
    }
}
