/// Returns the foreground and background colors for the first matching symbol.
fn rendered_cell_colors(terminal: &Terminal<TestBackend>, symbol: &str) -> (Color, Color) {
    let cell = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .find(|cell| cell.symbol() == symbol)
        .unwrap_or_else(|| panic!("rendered `{symbol}` cell"));

    (cell.fg, cell.bg)
}
