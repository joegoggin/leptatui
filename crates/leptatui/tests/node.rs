use leptatui::{RenderCtx, Result, block, text};
use ratatui::{Terminal, backend::TestBackend};

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
