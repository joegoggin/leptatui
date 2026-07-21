//! Deterministic text fallbacks for terminal image rendering.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Paragraph, Widget, Wrap},
};

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

/// Returns fallback text, preferring caller-provided alt text.
pub(crate) fn terminal_image_fallback_text(
    reason: TerminalImageFallback,
    alt: Option<&str>,
) -> String {
    alt.filter(|text| !text.is_empty())
        .unwrap_or_else(|| reason.message())
        .to_owned()
}

/// Renders deterministic fallback text into the provided buffer.
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
