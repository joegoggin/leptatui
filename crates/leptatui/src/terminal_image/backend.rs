//! Terminal image backend support.
//!
//! This module detects terminal graphics protocol support at runtime, renders
//! path-backed images when possible, and provides deterministic text fallback
//! when the active terminal or render target cannot display images.

use std::{
    fmt,
    path::Path,
    sync::{Arc, Mutex},
};

use ratatui::{
    buffer::Buffer,
    layout::{Rect, Size},
    widgets::Widget,
};
use ratatui_image::sliced::{SignedPosition, SlicedImage};

use super::{
    cache::TerminalImageCache,
    fallback::{TerminalImageFallback, TerminalImageRenderOutcome},
};

/// Runtime support for terminal image rendering.
#[derive(Clone)]
pub(crate) struct TerminalImageSupport {
    inner: TerminalImageSupportInner,
    cache: Arc<Mutex<TerminalImageCache>>,
}

/// Concrete terminal image support state.
#[derive(Clone, Debug)]
enum TerminalImageSupportInner {
    /// No graphics protocol was detected.
    Unavailable,
    /// A real terminal graphics protocol is available.
    Protocol(ratatui_image::picker::Picker),
}

impl Default for TerminalImageSupport {
    /// Returns fallback-only terminal image support.
    ///
    /// # Returns
    ///
    /// A [`TerminalImageSupport`] value that reports unavailable graphics
    /// protocol support.
    fn default() -> Self {
        Self::unavailable()
    }
}

impl fmt::Debug for TerminalImageSupport {
    /// Formats terminal image support without exposing cached protocol data.
    ///
    /// # Arguments
    ///
    /// * `formatter` — Formatter receiving the debug representation.
    ///
    /// # Returns
    ///
    /// A [`fmt::Result`] indicating whether formatting succeeded.
    ///
    /// # Errors
    ///
    /// Returns [`fmt::Error`] if writing to the formatter fails.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cached_images = self.cache.try_lock().map(|cache| cache.len()).ok();

        formatter
            .debug_struct("TerminalImageSupport")
            .field("inner", &self.inner)
            .field("cached_images", &cached_images)
            .finish()
    }
}

impl TerminalImageSupport {
    /// Detects terminal image support from stdio.
    ///
    /// # Returns
    ///
    /// A [`TerminalImageSupport`] value containing the detected image backend
    /// or deterministic fallback state.
    pub(crate) fn query_stdio() -> Self {
        query_stdio()
    }

    /// Returns support state for unavailable image support.
    ///
    /// # Returns
    ///
    /// A [`TerminalImageSupport`] value that falls back because no supported
    /// terminal image protocol is available.
    fn unavailable() -> Self {
        Self::with_inner(TerminalImageSupportInner::Unavailable)
    }

    /// Creates support state with an empty shared image cache.
    fn with_inner(inner: TerminalImageSupportInner) -> Self {
        Self {
            inner,
            cache: Arc::new(Mutex::new(TerminalImageCache::default())),
        }
    }

    /// Returns whether a terminal graphics protocol is available.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether this support state can attempt image
    /// protocol rendering.
    pub(crate) fn supports_protocol(&self) -> bool {
        matches!(self.inner, TerminalImageSupportInner::Protocol(_))
    }

    /// Renders a path-backed image into a Ratatui buffer when possible.
    ///
    /// # Arguments
    ///
    /// * `path` — Image file path to decode and render.
    /// * `area` — Terminal cell area assigned to the image.
    /// * `buffer` — Ratatui buffer receiving protocol output.
    ///
    /// # Returns
    ///
    /// A [`TerminalImageRenderOutcome`] describing whether protocol rendering
    /// succeeded or fallback text should be used.
    pub(crate) fn render_path_to_buffer(
        &self,
        path: &Path,
        area: Rect,
        buffer: &mut Buffer,
    ) -> TerminalImageRenderOutcome {
        if area.width == 0 || area.height == 0 {
            return TerminalImageRenderOutcome::Fallback(
                TerminalImageFallback::UnsupportedRenderTarget,
            );
        }

        self.render_path_to_buffer_sized(path, Size::new(area.width, area.height), 0, area, buffer)
    }

    /// Renders a cropped segment of a path-backed image into a Ratatui buffer.
    ///
    /// # Arguments
    ///
    /// * `path` — Image file path to decode and render.
    /// * `full_size` — Full terminal-cell size used before clipping.
    /// * `source_y` — Top row offset into the full image.
    /// * `area` — Terminal cell area assigned to the visible image segment.
    /// * `buffer` — Ratatui buffer receiving protocol output.
    ///
    /// # Returns
    ///
    /// A [`TerminalImageRenderOutcome`] describing whether protocol rendering
    /// succeeded or fallback text should be used.
    pub(crate) fn render_path_to_buffer_clipped(
        &self,
        path: &Path,
        full_size: Size,
        source_y: u16,
        area: Rect,
        buffer: &mut Buffer,
    ) -> TerminalImageRenderOutcome {
        if area.width == 0 || area.height == 0 || full_size.width == 0 || full_size.height == 0 {
            return TerminalImageRenderOutcome::Fallback(
                TerminalImageFallback::UnsupportedRenderTarget,
            );
        }

        self.render_path_to_buffer_sized(path, full_size, source_y, area, buffer)
    }

    /// Renders a path-backed image segment after size validation.
    fn render_path_to_buffer_sized(
        &self,
        path: &Path,
        full_size: Size,
        source_y: u16,
        area: Rect,
        buffer: &mut Buffer,
    ) -> TerminalImageRenderOutcome {
        match &self.inner {
            TerminalImageSupportInner::Unavailable => {
                TerminalImageRenderOutcome::Fallback(TerminalImageFallback::UnsupportedTerminal)
            }
            TerminalImageSupportInner::Protocol(picker) => render_cached_sliced_protocol(
                picker,
                &self.cache,
                path,
                full_size,
                source_y,
                area,
                buffer,
            ),
        }
    }
}

/// Queries stdio for supported terminal image protocols.
///
/// # Returns
///
/// A [`TerminalImageSupport`] value with a protocol picker when Kitty, Sixel,
/// or iTerm2 support is detected, otherwise fallback-only support.
fn query_stdio() -> TerminalImageSupport {
    use ratatui_image::picker::{Picker, ProtocolType};

    let Ok(picker) = Picker::from_query_stdio() else {
        return TerminalImageSupport::unavailable();
    };

    match picker.protocol_type() {
        ProtocolType::Sixel | ProtocolType::Kitty | ProtocolType::Iterm2 => {
            TerminalImageSupport::with_inner(TerminalImageSupportInner::Protocol(picker))
        }
        ProtocolType::Halfblocks => TerminalImageSupport::unavailable(),
    }
}

/// Renders a cached sliced protocol with a detected protocol picker.
fn render_cached_sliced_protocol(
    picker: &ratatui_image::picker::Picker,
    cache: &Arc<Mutex<TerminalImageCache>>,
    path: &Path,
    full_size: Size,
    source_y: u16,
    area: Rect,
    buffer: &mut Buffer,
) -> TerminalImageRenderOutcome {
    let Ok(mut cache) = cache.lock() else {
        return TerminalImageRenderOutcome::Fallback(TerminalImageFallback::RenderFailed);
    };
    let protocol = match cache.sliced_protocol(picker, path, full_size) {
        Ok(protocol) => protocol,
        Err(reason) => return TerminalImageRenderOutcome::Fallback(reason),
    };
    let y = -i16::try_from(source_y).unwrap_or(i16::MAX);

    SlicedImage::new(protocol, SignedPosition::from((0, y))).render(area, buffer);
    TerminalImageRenderOutcome::Rendered
}
