/// Returns the scroll offset from a layout view.
///
/// # Arguments
///
/// * `view` — Div or form view to inspect.
///
/// # Returns
///
/// A [`u16`] containing the current vertical scroll offset.
fn scroll_offset(view: &dyn View) -> u16 {
    view.style_metadata()
        .expect("expected styleable layout view")
        .scroll_offset()
}

/// Returns the position of a rendered terminal cell symbol.
///
/// # Arguments
///
/// * `terminal` — Test terminal containing the rendered buffer.
/// * `symbol` — Cell symbol to locate.
/// * `width` — Terminal width used to convert buffer index to coordinates.
///
/// # Returns
///
/// A [`tuple`](prim@tuple) containing the `x` and `y` coordinates.
///
/// # Panics
///
/// Panics if no rendered cell has the requested symbol.
fn symbol_position(terminal: &Terminal<TestBackend>, symbol: &str, width: u16) -> (u16, u16) {
    symbol_position_opt(terminal, symbol, width)
        .unwrap_or_else(|| panic!("rendered `{symbol}` cell"))
}

/// Returns the optional position of a rendered terminal cell symbol.
///
/// # Arguments
///
/// * `terminal` — Test terminal containing the rendered buffer.
/// * `symbol` — Cell symbol to locate.
/// * `width` — Terminal width used to convert buffer index to coordinates.
///
/// # Returns
///
/// An [`Option`] containing the `x` and `y` coordinates when the symbol exists.
fn symbol_position_opt(
    terminal: &Terminal<TestBackend>,
    symbol: &str,
    width: u16,
) -> Option<(u16, u16)> {
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .enumerate()
        .find_map(|(index, cell)| {
            (cell.symbol() == symbol).then(|| {
                let index = index as u16;
                (index % width, index / width)
            })
        })
}

/// Returns the symbol rendered at a terminal coordinate.
///
/// # Arguments
///
/// * `terminal` — Test terminal containing the rendered buffer.
/// * `x` — Horizontal cell coordinate.
/// * `y` — Vertical cell coordinate.
/// * `width` — Terminal width used to convert coordinates to a buffer index.
///
/// # Returns
///
/// A string slice containing the rendered cell symbol.
fn cell_symbol(terminal: &Terminal<TestBackend>, x: u16, y: u16, width: u16) -> &str {
    let index = usize::from(y) * usize::from(width) + usize::from(x);
    terminal.backend().buffer().content()[index].symbol()
}

/// Returns foreground and background colors at a terminal coordinate.
///
/// # Arguments
///
/// * `terminal` — Test terminal containing the rendered buffer.
/// * `x` — Horizontal cell coordinate.
/// * `y` — Vertical cell coordinate.
/// * `width` — Terminal width used to convert coordinates to a buffer index.
///
/// # Returns
///
/// A [`tuple`](prim@tuple) containing foreground and background colors.
fn cell_colors(terminal: &Terminal<TestBackend>, x: u16, y: u16, width: u16) -> (Color, Color) {
    let index = usize::from(y) * usize::from(width) + usize::from(x);
    let cell = &terminal.backend().buffer().content()[index];
    (cell.fg, cell.bg)
}

/// Returns text modifiers at a terminal coordinate.
///
/// # Arguments
///
/// * `terminal` — Test terminal containing the rendered buffer.
/// * `x` — Horizontal cell coordinate.
/// * `y` — Vertical cell coordinate.
/// * `width` — Terminal width used to convert coordinates to a buffer index.
///
/// # Returns
///
/// A [`Modifier`] value containing the rendered cell modifiers.
fn cell_modifiers(terminal: &Terminal<TestBackend>, x: u16, y: u16, width: u16) -> Modifier {
    let index = usize::from(y) * usize::from(width) + usize::from(x);
    terminal.backend().buffer().content()[index].modifier
}
/// Measures a view's intrinsic height at the current test viewport width.
///
/// # Arguments
///
/// * `view` — View whose two-axis measurement supplies the height.
/// * `ctx` — Rendering context containing the test viewport.
///
/// # Returns
///
/// A saturated `u16` intrinsic height.
fn intrinsic_height(view: &dyn View, ctx: &mut RenderCtx<'_, '_>) -> u16 {
    let area = ctx.area();
    let measured = view.measure(
        LayoutSize::new(Some(f32::from(area.width)), None),
        LayoutSize::new(
            AvailableSpace::Definite(f32::from(area.width)),
            AvailableSpace::Definite(f32::from(area.height)),
        ),
        ctx,
    );
    measured.height.clamp(0.0, f32::from(u16::MAX)).floor() as u16
}
