//! Tests for image capability detection, fallback rendering, and sizing.

use std::path::Path;

use ratatui::{Terminal, backend::TestBackend, layout::Size, style::Style};

use crate::{component::RenderCtx, context};

use super::{TerminalImageFallback, TerminalImageRenderOutcome};
use crate::terminal_image::fallback::terminal_image_fallback_text;

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
                    0,
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

/// Verifies clipped fallback text starts at the requested source column.
///
/// # Example Under Test
///
/// ```text
/// alt = "left-right"
/// full_size = 10x1
/// source_x = 5
/// target = 5x1
/// ```
///
/// # Assertions
///
/// - The visible fallback slice contains `right`.
/// - The skipped fallback prefix is not rendered.
#[test]
fn clipped_fallback_starts_at_source_column() {
    let backend = TestBackend::new(5, 1);
    let mut terminal = Terminal::new(backend).expect("test terminal should initialize");

    terminal
        .draw(|frame| {
            context::hooks::__with_context_scope(|| {
                let mut ctx = RenderCtx::new(frame);
                ctx.render_terminal_image_path_clipped(
                    Path::new("missing.png"),
                    Some("left-right"),
                    Style::default(),
                    Size::new(10, 1),
                    5,
                    0,
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

    assert!(rendered.contains("right"));
    assert!(!rendered.contains("left"));
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
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 10, 73, 68, 65, 84, 120, 156, 99, 0, 1, 0, 0, 5, 0, 1,
        13, 10, 45, 180, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
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
    let resized = Resize::Fit(None).resize(&decoded, FontSize::new(10, 20), Size::new(2, 2), None);
    let _ = std::fs::remove_file(path);

    assert_eq!(resized.width(), 20);
    assert_eq!(resized.height(), 40);
}
