//! Terminal-image rendering integration for [`RenderCtx`].

use std::path::Path;

use ratatui::{
    buffer::Buffer,
    layout::{Rect, Size},
    style::Style,
};

use crate::terminal_image::{
    TerminalImageFallback, TerminalImageRenderOutcome, TerminalImageSupport,
    render_terminal_image_fallback,
};

use super::RenderCtx;

impl RenderCtx<'_, '_> {
    /// Renders a path-backed terminal image or deterministic fallback text.
    #[allow(dead_code)]
    pub(crate) fn render_terminal_image_path(
        &mut self,
        path: &Path,
        alt: Option<&str>,
        fallback_style: Style,
    ) -> TerminalImageRenderOutcome {
        self.render_terminal_image_or_fallback(
            alt,
            fallback_style,
            |terminal_images, area, buffer| {
                terminal_images.render_path_to_buffer(path, area, buffer)
            },
        )
    }

    /// Renders a cropped segment of a path-backed terminal image.
    pub(crate) fn render_terminal_image_path_clipped(
        &mut self,
        path: &Path,
        alt: Option<&str>,
        fallback_style: Style,
        full_size: Size,
        source_y: u16,
    ) -> TerminalImageRenderOutcome {
        if !self.target.supports_terminal_images() {
            let reason = TerminalImageFallback::UnsupportedRenderTarget;
            self.render_terminal_image_fallback_clipped(
                reason,
                alt,
                fallback_style,
                full_size,
                source_y,
            );
            return TerminalImageRenderOutcome::Fallback(reason);
        }

        let area = self.area;
        let terminal_images = self.terminal_images.clone();
        let outcome = terminal_images.render_path_to_buffer_clipped(
            path,
            full_size,
            source_y,
            area,
            self.target.buffer_mut(),
        );
        if let TerminalImageRenderOutcome::Fallback(reason) = outcome {
            self.render_terminal_image_fallback_clipped(
                reason,
                alt,
                fallback_style,
                full_size,
                source_y,
            );
        }

        outcome
    }

    /// Runs a terminal-image render operation and writes fallback text when needed.
    fn render_terminal_image_or_fallback(
        &mut self,
        alt: Option<&str>,
        fallback_style: Style,
        render: impl FnOnce(&TerminalImageSupport, Rect, &mut Buffer) -> TerminalImageRenderOutcome,
    ) -> TerminalImageRenderOutcome {
        if !self.target.supports_terminal_images() {
            let reason = TerminalImageFallback::UnsupportedRenderTarget;
            self.render_terminal_image_fallback(reason, alt, fallback_style);
            return TerminalImageRenderOutcome::Fallback(reason);
        }

        let outcome = render(&self.terminal_images, self.area, self.target.buffer_mut());
        if let TerminalImageRenderOutcome::Fallback(reason) = outcome {
            self.render_terminal_image_fallback(reason, alt, fallback_style);
        }

        outcome
    }

    /// Writes deterministic fallback text for a terminal-image render failure.
    fn render_terminal_image_fallback(
        &mut self,
        reason: TerminalImageFallback,
        alt: Option<&str>,
        fallback_style: Style,
    ) {
        render_terminal_image_fallback(
            reason,
            alt,
            fallback_style,
            self.area,
            self.target.buffer_mut(),
        );
    }

    /// Writes fallback text into the visible slice of a clipped image area.
    fn render_terminal_image_fallback_clipped(
        &mut self,
        reason: TerminalImageFallback,
        alt: Option<&str>,
        fallback_style: Style,
        full_size: Size,
        source_y: u16,
    ) {
        if full_size.width == 0 || full_size.height == 0 || source_y >= full_size.height {
            return;
        }

        let full_area = Rect::new(0, 0, full_size.width, full_size.height);
        let mut buffer = Buffer::empty(full_area);
        render_terminal_image_fallback(reason, alt, fallback_style, full_area, &mut buffer);

        let area = self.area;
        let width = area.width.min(full_size.width);
        let height = area.height.min(full_size.height.saturating_sub(source_y));
        let target = self.target.buffer_mut();

        for y in 0..height {
            for x in 0..width {
                let source = buffer[(x, source_y.saturating_add(y))].clone();
                let destination = (area.x.saturating_add(x), area.y.saturating_add(y));
                if let Some(cell) = target.cell_mut(destination) {
                    *cell = source;
                }
            }
        }
    }
}
