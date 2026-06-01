//! Node rendering tests.
//!
//! These tests render node trees against Ratatui's test backend and inspect the
//! resulting terminal buffer.

use leptatui::{RenderCtx, Result, block, text};
use ratatui::{Terminal, backend::TestBackend};

/// Verifies a block node renders its child text.
///
/// # Example Under Test
///
/// ```text
/// block(text("Hello"))
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The node render call succeeds.
/// - The rendered buffer contains `Hello`.
#[test]
fn renders_block_and_text_nodes() -> Result<()> {
    let backend = TestBackend::new(24, 5);
    let mut terminal = Terminal::new(backend)?;
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = block(text("Hello")).render(&mut ctx);
    })?;
    render_result?;

    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();

    assert!(rendered.contains("Hello"));

    Ok(())
}
