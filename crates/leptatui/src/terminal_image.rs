//! Terminal image backend support.
//!
//! This module detects terminal graphics protocol support at runtime, renders
//! path-backed images when possible, and provides deterministic text fallback
//! when the active terminal or render target cannot display images.

use std::{
    collections::{HashMap, hash_map::Entry},
    fmt,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use ratatui::{
    buffer::Buffer,
    layout::{Rect, Size},
    style::Style,
    widgets::{Paragraph, Widget, Wrap},
};
use ratatui_image::sliced::{SignedPosition, SlicedImage, SlicedProtocol};

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

/// Cached terminal image protocol data reused across redraws.
#[derive(Default)]
struct TerminalImageCache {
    sliced_protocols: HashMap<TerminalImageCacheKey, SlicedProtocol>,
}

/// Cache key for a path-backed image rendered at a fixed terminal-cell size.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct TerminalImageCacheKey {
    path: PathBuf,
    width: u16,
    height: u16,
}

/// Result of trying to render a terminal image.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TerminalImageRenderOutcome {
    /// Image protocol data was written into the Ratatui buffer.
    Rendered,
    /// Deterministic fallback text should be shown instead.
    Fallback(TerminalImageFallback),
}

/// Reason terminal image rendering fell back to text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TerminalImageFallback {
    /// The current terminal does not expose a supported image protocol.
    UnsupportedTerminal,
    /// The active render target is not a real terminal frame.
    UnsupportedRenderTarget,
    /// The image source could not be decoded.
    DecodeFailed,
    /// The image could not be encoded for the detected protocol.
    RenderFailed,
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
        let cached_images = self
            .cache
            .try_lock()
            .map(|cache| cache.sliced_protocols.len())
            .ok();

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

/// Returns fallback text, preferring caller-provided alt text.
///
/// # Arguments
///
/// * `reason` — Fallback reason used when no alt text is available.
/// * `alt` — Optional caller-provided fallback text.
///
/// # Returns
///
/// A [`String`] containing alt text or a deterministic reason message.
pub(crate) fn terminal_image_fallback_text(
    reason: TerminalImageFallback,
    alt: Option<&str>,
) -> String {
    alt.filter(|text| !text.is_empty())
        .unwrap_or_else(|| reason.message())
        .to_owned()
}

/// Renders deterministic fallback text into the provided buffer.
///
/// # Arguments
///
/// * `reason` — Fallback reason used when no alt text is available.
/// * `alt` — Optional caller-provided fallback text.
/// * `style` — Ratatui style applied to the fallback paragraph.
/// * `area` — Terminal cell area assigned to the fallback paragraph.
/// * `buffer` — Ratatui buffer receiving fallback text.
pub(crate) fn render_terminal_image_fallback(
    reason: TerminalImageFallback,
    alt: Option<&str>,
    style: Style,
    area: Rect,
    buffer: &mut Buffer,
) {
    Paragraph::new(terminal_image_fallback_text(reason, alt))
        .style(style)
        .wrap(Wrap { trim: false })
        .render(area, buffer);
}

impl TerminalImageFallback {
    /// Returns a deterministic fallback message for this reason.
    ///
    /// # Returns
    ///
    /// A static string describing why image rendering fell back to text.
    fn message(self) -> &'static str {
        match self {
            Self::UnsupportedTerminal | Self::UnsupportedRenderTarget => {
                "terminal image support is unavailable"
            }
            Self::DecodeFailed => "image could not be decoded",
            Self::RenderFailed => "image could not be rendered",
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

impl TerminalImageCache {
    /// Returns a cached sliced protocol for the image path and requested size.
    fn sliced_protocol(
        &mut self,
        picker: &ratatui_image::picker::Picker,
        path: &Path,
        size: Size,
    ) -> Result<&SlicedProtocol, TerminalImageFallback> {
        let key = TerminalImageCacheKey {
            path: path.to_path_buf(),
            width: size.width,
            height: size.height,
        };

        match self.sliced_protocols.entry(key) {
            Entry::Occupied(entry) => Ok(entry.into_mut()),
            Entry::Vacant(entry) => {
                let reader = image::ImageReader::open(path)
                    .map_err(|_| TerminalImageFallback::DecodeFailed)?;
                let image = reader
                    .decode()
                    .map_err(|_| TerminalImageFallback::DecodeFailed)?;
                let protocol = SlicedProtocol::new(picker, image, Some(size))
                    .map_err(|_| TerminalImageFallback::RenderFailed)?;
                Ok(entry.insert(protocol))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use ratatui::{Terminal, backend::TestBackend, layout::Size, style::Style};

    use crate::{component::RenderCtx, context};

    use super::{TerminalImageFallback, TerminalImageRenderOutcome, terminal_image_fallback_text};

    /// Verifies alt text overrides fallback reason messages.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// reason = UnsupportedTerminal
    /// alt = "logo"
    /// ```
    ///
    /// # Assertions
    ///
    /// - The returned fallback text is the provided alt text.
    #[test]
    fn fallback_text_prefers_alt_text() {
        assert_eq!(
            terminal_image_fallback_text(TerminalImageFallback::UnsupportedTerminal, Some("logo")),
            "logo"
        );
    }

    /// Verifies fallback reasons provide deterministic text without alt text.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// reason = DecodeFailed
    /// alt = None
    /// ```
    ///
    /// # Assertions
    ///
    /// - The returned fallback text describes the decode failure.
    #[test]
    fn fallback_text_uses_reason_when_alt_is_missing() {
        assert_eq!(
            terminal_image_fallback_text(TerminalImageFallback::DecodeFailed, None),
            "image could not be decoded"
        );
    }

    /// Verifies test backends render fallback text instead of protocol output.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// backend = TestBackend
    /// path = missing.png
    /// alt = "fallback image"
    /// ```
    ///
    /// # Assertions
    ///
    /// - Rendering reports a fallback outcome.
    /// - The rendered buffer contains the fallback text.
    /// - The rendered buffer does not contain escape characters.
    ///
    /// # Why
    ///
    /// Test rendering must remain deterministic and must not write raw terminal
    /// graphics protocol data into normal text buffers.
    #[test]
    fn test_backend_renders_text_fallback_without_protocol_output() {
        let backend = TestBackend::new(32, 3);
        let mut terminal = Terminal::new(backend).expect("test terminal should initialize");
        let mut outcome = None;

        terminal
            .draw(|frame| {
                context::hooks::__with_context_scope(|| {
                    let mut ctx = RenderCtx::new(frame);
                    outcome = Some(ctx.render_terminal_image_path(
                        Path::new("missing.png"),
                        Some("fallback image"),
                        Style::default(),
                    ));
                });
            })
            .expect("rendering should succeed");

        assert!(matches!(
            outcome,
            Some(TerminalImageRenderOutcome::Fallback(_))
        ));

        let rendered = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(rendered.contains("fallback image"));
        assert!(!rendered.contains('\u{1b}'));
    }

    /// Verifies clipped fallback text starts at the requested source row.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// alt = "top\nbottom"
    /// full_size = 16x2
    /// source_y = 1
    /// target = 16x1
    /// ```
    ///
    /// # Assertions
    ///
    /// - The visible fallback slice contains the second fallback row.
    /// - The skipped fallback row is not rendered.
    #[test]
    fn clipped_fallback_starts_at_source_row() {
        let backend = TestBackend::new(16, 1);
        let mut terminal = Terminal::new(backend).expect("test terminal should initialize");

        terminal
            .draw(|frame| {
                context::hooks::__with_context_scope(|| {
                    let mut ctx = RenderCtx::new(frame);
                    ctx.render_terminal_image_path_clipped(
                        Path::new("missing.png"),
                        Some("top\nbottom"),
                        Style::default(),
                        Size::new(16, 2),
                        1,
                    );
                });
            })
            .expect("rendering should succeed");

        let rendered = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(rendered.contains("bottom"));
        assert!(!rendered.contains("top"));
    }

    /// Verifies image decoding and fit sizing are deterministic.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// 1x1 PNG
    /// area = 2x2 cells
    /// font = 10x20 pixels
    /// ```
    ///
    /// # Assertions
    ///
    /// - The PNG fixture writes, opens, and decodes successfully.
    /// - `Resize::Fit` returns an image matching the target pixel area.
    #[test]
    fn decode_and_fit_sizing_are_deterministic() {
        use ratatui_image::{FontSize, Resize};

        /// Encoded 1x1 PNG fixture bytes.
        const ONE_BY_ONE_PNG: &[u8] = &[
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1,
            8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 10, 73, 68, 65, 84, 120, 156, 99, 0, 1, 0, 0,
            5, 0, 1, 13, 10, 45, 180, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
        ];
        let path = std::env::temp_dir().join(format!(
            "leptatui-image-decode-fit-{}.png",
            std::process::id()
        ));

        std::fs::write(&path, ONE_BY_ONE_PNG).expect("png fixture should be written");
        let decoded = image::ImageReader::open(&path)
            .expect("png fixture should open")
            .decode()
            .expect("png fixture should decode");
        let resized =
            Resize::Fit(None).resize(&decoded, FontSize::new(10, 20), Size::new(2, 2), None);
        let _ = std::fs::remove_file(path);

        assert_eq!(resized.width(), 20);
        assert_eq!(resized.height(), 40);
    }
}
