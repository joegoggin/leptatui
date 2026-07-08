//! Optional terminal image backend support.
//!
//! This module keeps image decoding and terminal graphics protocol rendering
//! behind the `images` feature. Public image view APIs are layered on top of
//! this crate-internal backend.

use std::path::Path;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Paragraph, Widget, Wrap},
};

/// Runtime support for terminal image rendering.
#[derive(Clone, Debug)]
pub(crate) struct TerminalImageSupport {
    inner: TerminalImageSupportInner,
}

/// Concrete terminal image support state.
#[derive(Clone, Debug)]
#[cfg_attr(not(feature = "images"), allow(dead_code))]
enum TerminalImageSupportInner {
    /// The crate was built without the `images` feature.
    #[cfg(not(feature = "images"))]
    FeatureDisabled,
    /// The feature is enabled but no graphics protocol was detected.
    Unavailable,
    /// A real terminal graphics protocol is available.
    #[cfg(feature = "images")]
    Protocol(ratatui_image::picker::Picker),
}

/// Result of trying to render a terminal image.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(not(feature = "images"), allow(dead_code))]
pub(crate) enum TerminalImageRenderOutcome {
    /// Image protocol data was written into the Ratatui buffer.
    Rendered,
    /// Deterministic fallback text should be shown instead.
    Fallback(TerminalImageFallback),
}

/// Reason terminal image rendering fell back to text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(not(feature = "images"), allow(dead_code))]
pub(crate) enum TerminalImageFallback {
    /// The crate was built without optional image support.
    #[cfg(not(feature = "images"))]
    FeatureDisabled,
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
    fn default() -> Self {
        #[cfg(feature = "images")]
        {
            Self::unavailable()
        }

        #[cfg(not(feature = "images"))]
        {
            Self::feature_disabled()
        }
    }
}

impl TerminalImageSupport {
    /// Detects terminal image support from stdio.
    ///
    /// The query is intentionally feature-gated so default builds avoid image
    /// decoding and terminal graphics protocol dependencies.
    ///
    /// # Returns
    ///
    /// A [`TerminalImageSupport`] value containing the detected image backend
    /// or deterministic fallback state.
    pub(crate) fn query_stdio() -> Self {
        query_stdio()
    }

    /// Returns support state for a build without image feature support.
    ///
    /// # Returns
    ///
    /// A [`TerminalImageSupport`] value that always falls back because the
    /// crate was built without optional image dependencies.
    #[cfg(not(feature = "images"))]
    fn feature_disabled() -> Self {
        Self {
            inner: TerminalImageSupportInner::FeatureDisabled,
        }
    }

    /// Returns support state for feature-enabled but unavailable image support.
    ///
    /// # Returns
    ///
    /// A [`TerminalImageSupport`] value that falls back because no supported
    /// terminal image protocol is available.
    #[cfg_attr(not(feature = "images"), allow(dead_code))]
    fn unavailable() -> Self {
        Self {
            inner: TerminalImageSupportInner::Unavailable,
        }
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
    #[cfg_attr(not(feature = "images"), allow(unused_variables))]
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

        match &self.inner {
            #[cfg(not(feature = "images"))]
            TerminalImageSupportInner::FeatureDisabled => {
                TerminalImageRenderOutcome::Fallback(TerminalImageFallback::FeatureDisabled)
            }
            TerminalImageSupportInner::Unavailable => {
                TerminalImageRenderOutcome::Fallback(TerminalImageFallback::UnsupportedTerminal)
            }
            #[cfg(feature = "images")]
            TerminalImageSupportInner::Protocol(picker) => {
                render_path_with_picker(picker, path, area, buffer)
            }
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
            #[cfg(not(feature = "images"))]
            Self::FeatureDisabled => "image support is disabled",
            Self::UnsupportedTerminal | Self::UnsupportedRenderTarget => {
                "terminal image support is unavailable"
            }
            Self::DecodeFailed => "image could not be decoded",
            Self::RenderFailed => "image could not be rendered",
        }
    }
}

#[cfg(not(feature = "images"))]
/// Returns feature-disabled image support.
///
/// # Returns
///
/// A [`TerminalImageSupport`] value that always falls back.
fn query_stdio() -> TerminalImageSupport {
    TerminalImageSupport::feature_disabled()
}

#[cfg(feature = "images")]
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
        ProtocolType::Sixel | ProtocolType::Kitty | ProtocolType::Iterm2 => TerminalImageSupport {
            inner: TerminalImageSupportInner::Protocol(picker),
        },
        ProtocolType::Halfblocks => TerminalImageSupport::unavailable(),
    }
}

#[cfg(feature = "images")]
/// Renders a path-backed image with a detected protocol picker.
///
/// # Arguments
///
/// * `picker` — Detected protocol and font-size information.
/// * `path` — Image file path to decode.
/// * `area` — Terminal cell area assigned to the image.
/// * `buffer` — Ratatui buffer receiving protocol output.
///
/// # Returns
///
/// A [`TerminalImageRenderOutcome`] describing whether protocol rendering
/// succeeded or fallback text should be used.
fn render_path_with_picker(
    picker: &ratatui_image::picker::Picker,
    path: &Path,
    area: Rect,
    buffer: &mut Buffer,
) -> TerminalImageRenderOutcome {
    use ratatui::{layout::Size, widgets::Widget};
    use ratatui_image::{Image as RatatuiImage, Resize};

    let reader = match image::ImageReader::open(path) {
        Ok(reader) => reader,
        Err(_) => {
            return TerminalImageRenderOutcome::Fallback(TerminalImageFallback::DecodeFailed);
        }
    };
    let image = match reader.decode() {
        Ok(image) => image,
        Err(_) => {
            return TerminalImageRenderOutcome::Fallback(TerminalImageFallback::DecodeFailed);
        }
    };

    let protocol =
        match picker.new_protocol(image, Size::new(area.width, area.height), Resize::Fit(None)) {
            Ok(protocol) => protocol,
            Err(_) => {
                return TerminalImageRenderOutcome::Fallback(TerminalImageFallback::RenderFailed);
            }
        };

    if protocol.needs_placeholder(area).is_some() {
        return TerminalImageRenderOutcome::Fallback(TerminalImageFallback::RenderFailed);
    }

    RatatuiImage::new(&protocol)
        .allow_clipping(true)
        .render(area, buffer);

    TerminalImageRenderOutcome::Rendered
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use ratatui::{Terminal, backend::TestBackend, style::Style};

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
}
