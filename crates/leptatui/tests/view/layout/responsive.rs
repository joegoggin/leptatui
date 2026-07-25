/// Verifies render contexts apply media rules using the root viewport.
///
/// # Example Under Test
///
/// ```text
/// @media (max-width: 12) { .accent { fg: Yellow } }
/// terminal width = 12
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The view render call succeeds.
/// - The rendered text resolves the media-rule foreground color.
#[test]
fn render_context_applies_media_rules_from_root_viewport() -> Result<()> {
    let backend = TestBackend::new(12, 3);
    let mut terminal = Terminal::new(backend)?;
    let view = text("Hi").with_classes("accent");
    let stylesheet = Stylesheet::new().media_rule(
        MediaQuery::max_width(12),
        StyleSelector::class("accent"),
        TuiStyle::new().foreground(Color::Yellow),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    let cell = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .find(|cell| cell.symbol() == "H")
        .expect("rendered H cell");

    assert_eq!(cell.fg, Color::Yellow);

    Ok(())
}

/// Verifies media flex direction gives stacked bordered buttons enough height.
///
/// # Example Under Test
///
/// ```text
/// div([button("A"), button("B")]).stack
/// @media (max-width: 12) { .stack { flex_direction: Column } }
/// ```
///
/// # Assertions
///
/// - The styled render succeeds.
/// - The first bordered button renders near the top.
/// - The second bordered button renders lower after column stacking.
#[test]
fn media_direction_gives_stacked_bordered_buttons_minimum_height() -> Result<()> {
    let backend = TestBackend::new(12, 6);
    let mut terminal = Terminal::new(backend)?;
    let view = div(vec![button("A"), button("B")]).with_classes("stack");
    let stylesheet = Stylesheet::new()
        .rule(
            StyleSelector::class("stack"),
            TuiStyle::new().display(Display::Flex),
        )
        .media_rule(
            MediaQuery::max_width(12),
            StyleSelector::class("stack"),
            TuiStyle::new().flex_direction(FlexDirection::Column),
        );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert_eq!(symbol_position(&terminal, "A", 12).1, 1);
    assert_eq!(symbol_position(&terminal, "B", 12).1, 4);

    Ok(())
}

/// Verifies columns reserve height for nested media-stacked bordered buttons.
///
/// # Example Under Test
///
/// ```text
/// div([text("Top"), div(buttons).stack, text("End")])
/// @media (max-width: 12) { .stack { flex_direction: Column } }
/// ```
///
/// # Assertions
///
/// - The styled render succeeds.
/// - The fourth nested button renders on the expected lower row.
#[test]
fn column_reserves_height_for_nested_stacked_bordered_buttons() -> Result<()> {
    let backend = TestBackend::new(12, 14);
    let mut terminal = Terminal::new(backend)?;
    let view = div((
        text("Top"),
        div(vec![button("A"), button("B"), button("C"), button("D")]).with_classes("stack"),
        text("End"),
    ));
    let stylesheet = Stylesheet::new()
        .rule(
            StyleSelector::class("stack"),
            TuiStyle::new().display(Display::Flex),
        )
        .media_rule(
            MediaQuery::max_width(12),
            StyleSelector::class("stack"),
            TuiStyle::new().flex_direction(FlexDirection::Column),
        );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert_eq!(symbol_position(&terminal, "D", 12).1, 11);

    Ok(())
}
