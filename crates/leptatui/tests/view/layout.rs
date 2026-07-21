/// Verifies fitting columns do not render a scrollbar.
///
/// # Example Under Test
///
/// ```text
/// column([text("12345678")])
/// terminal size = 8x1
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The view render call succeeds.
/// - The rightmost cell remains the final text character.
#[test]
fn fitting_column_does_not_render_scrollbar() -> Result<()> {
    let backend = TestBackend::new(8, 1);
    let mut terminal = Terminal::new(backend)?;
    let view = column([text("12345678")]);
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(cell_symbol(&terminal, 7, 0, 8), "8");

    Ok(())
}

/// Verifies overflowing columns render a right-side scrollbar.
///
/// # Example Under Test
///
/// ```text
/// column([text("One"), text("Two"), text("Three")])
/// terminal size = 8x2
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The view render call succeeds.
/// - The first scrollbar cell renders as the scroll thumb.
/// - The second scrollbar cell renders as the scrollbar track.
#[test]
fn overflowing_column_renders_right_scrollbar() -> Result<()> {
    let backend = TestBackend::new(8, 2);
    let mut terminal = Terminal::new(backend)?;
    let view = column(vec![text("One"), text("Two"), text("Three")]);
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(cell_symbol(&terminal, 7, 0, 8), symbol_block::FULL);
    assert_eq!(
        cell_symbol(&terminal, 7, 1, 8),
        symbol_line::DOUBLE_VERTICAL
    );

    Ok(())
}

/// Verifies dynamic overflowing columns keep scroll metadata between refreshes.
///
/// # Example Under Test
///
/// ```text
/// dynamic(|| column([text("One"), text("Two"), text("Three")]))
/// terminal size = 8x2
/// ```
///
/// # Assertions
///
/// - Initial rendering measures overflow.
/// - The Down key is handled by the refreshed dynamic child.
/// - Rendering after the key shows the scrolled second row.
#[test]
fn dynamic_overflowing_column_scrolls_after_render() -> Result<()> {
    let backend = TestBackend::new(8, 2);
    let mut terminal = Terminal::new(backend)?;
    let mut view = dynamic(|| column(vec![text("One"), text("Two"), text("Three")]));

    draw_view(&mut terminal, &view)?;
    assert_eq!(cell_symbol(&terminal, 0, 0, 8), "O");

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Down))?,
        KeyControl::Handled
    );
    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 0, 0, 8), "T");
    assert_eq!(cell_symbol(&terminal, 1, 0, 8), "w");

    Ok(())
}

/// Verifies overflowing columns reserve width for the scrollbar.
///
/// # Example Under Test
///
/// ```text
/// column([text("123456"), text("more"), text("tail")])
/// terminal size = 6x2
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The view render call succeeds.
/// - Text wraps before the scrollbar column.
/// - The scrollbar thumb occupies the rightmost column.
#[test]
fn overflowing_column_reserves_width_for_scrollbar() -> Result<()> {
    let backend = TestBackend::new(6, 2);
    let mut terminal = Terminal::new(backend)?;
    let view = column(vec![text("123456"), text("more"), text("tail")]);
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(cell_symbol(&terminal, 4, 0, 6), "5");
    assert_eq!(cell_symbol(&terminal, 5, 0, 6), symbol_block::FULL);
    assert_eq!(cell_symbol(&terminal, 0, 1, 6), "6");

    Ok(())
}

/// Verifies overflowing columns update the scrollbar thumb after scrolling.
///
/// # Example Under Test
///
/// ```text
/// column([text("One"), text("Two"), text("Three")])
/// PageDown
/// ```
///
/// # Assertions
///
/// - The initial terminal draw succeeds.
/// - PageDown is handled by the view.
/// - The second terminal draw succeeds.
/// - The scrollbar thumb moves from the top cell to the bottom cell.
#[test]
fn overflowing_column_updates_scrollbar_position() -> Result<()> {
    let backend = TestBackend::new(8, 2);
    let mut terminal = Terminal::new(backend)?;
    let mut view = column(vec![text("One"), text("Two"), text("Three")]);
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(
        cell_symbol(&terminal, 7, 0, 8),
        symbol_line::DOUBLE_VERTICAL
    );
    assert_eq!(cell_symbol(&terminal, 7, 1, 8), symbol_block::FULL);

    Ok(())
}

/// Verifies overflowing column scrollbars reach the bottom at max scroll.
///
/// # Example Under Test
///
/// ```text
/// column(Line 0..Line 9)
/// PageDown
/// ```
///
/// # Assertions
///
/// - The initial terminal draw succeeds.
/// - PageDown is handled by the view.
/// - The second terminal draw succeeds.
/// - The scrollbar thumb reaches the bottom row.
#[test]
fn overflowing_column_scrollbar_reaches_bottom_at_max_scroll() -> Result<()> {
    let backend = TestBackend::new(8, 5);
    let mut terminal = Terminal::new(backend)?;
    let children = (0..10).map(|index| text(format!("Line {index}")));
    let mut view = column(children.collect::<Vec<_>>());
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(cell_symbol(&terminal, 7, 4, 8), symbol_block::FULL);

    Ok(())
}

/// Verifies Vim `G` scrolls an overflowing column to the bottom.
///
/// # Example Under Test
///
/// ```text
/// column(Line 0..Line 9)
/// G
/// ```
///
/// # Assertions
///
/// - The initial draw succeeds.
/// - The initial scroll offset is zero.
/// - `G` is handled by the view.
/// - The scroll offset moves to the bottom.
#[test]
fn overflowing_column_scrolls_to_bottom_with_vim_g() -> Result<()> {
    let backend = TestBackend::new(8, 5);
    let mut terminal = Terminal::new(backend)?;
    let children = (0..10).map(|index| text(format!("Line {index}")));
    let mut view = column(children.collect::<Vec<_>>());

    draw_view(&mut terminal, &view)?;

    assert_eq!(scroll_offset(&view), 0);
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    assert_eq!(scroll_offset(&view), 5);

    Ok(())
}

/// Verifies Vim `gg` scrolls an overflowing column to the top.
///
/// # Example Under Test
///
/// ```text
/// column(Line 0..Line 9)
/// G, g, g
/// ```
///
/// # Assertions
///
/// - The initial draw succeeds.
/// - `G` scrolls to the bottom.
/// - The first `g` keeps the pending top-scroll prefix.
/// - The second `g` scrolls to the top.
#[test]
fn overflowing_column_scrolls_to_top_with_vim_gg() -> Result<()> {
    let backend = TestBackend::new(8, 5);
    let mut terminal = Terminal::new(backend)?;
    let children = (0..10).map(|index| text(format!("Line {index}")));
    let mut view = column(children.collect::<Vec<_>>());

    draw_view(&mut terminal, &view)?;
    view.handle_key_event(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE))?;
    assert_eq!(scroll_offset(&view), 5);

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    assert_eq!(scroll_offset(&view), 5);
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    assert_eq!(scroll_offset(&view), 0);

    Ok(())
}

/// Verifies the Vim `gg` prefix resets after an unrelated key.
///
/// # Example Under Test
///
/// ```text
/// G, g, Down, g, g
/// ```
///
/// # Assertions
///
/// - The initial draw succeeds.
/// - `G` scrolls to the bottom.
/// - `g`, `Down`, `g` leaves the scroll offset at the bottom.
/// - A fresh `g` completes the prefix and scrolls to the top.
#[test]
fn vim_scroll_to_top_prefix_resets_on_unrelated_key() -> Result<()> {
    let backend = TestBackend::new(8, 5);
    let mut terminal = Terminal::new(backend)?;
    let children = (0..10).map(|index| text(format!("Line {index}")));
    let mut view = column(children.collect::<Vec<_>>());

    draw_view(&mut terminal, &view)?;
    view.handle_key_event(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE))?;
    assert_eq!(scroll_offset(&view), 5);

    view.handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))?;
    view.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))?;
    view.handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))?;
    assert_eq!(scroll_offset(&view), 5);

    view.handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))?;
    assert_eq!(scroll_offset(&view), 0);

    Ok(())
}

/// Verifies parent backgrounds fill the bottom row after scrolling down.
///
/// # Example Under Test
///
/// ```text
/// column([text("Top"), button("Launch"), text("Tail")]).surface
/// Down
/// ```
///
/// # Assertions
///
/// - The initial styled render succeeds.
/// - The down key is handled by the view.
/// - The second styled render succeeds.
/// - Empty and occupied cells on the bottom row keep the parent background.
#[test]
fn overflowing_column_keeps_parent_background_on_bottom_row_after_scrolling_down() -> Result<()> {
    let backend = TestBackend::new(12, 2);
    let mut terminal = Terminal::new(backend)?;
    let mut view = column((text("Top"), button("Launch"), text("Tail"))).with_classes("surface");
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::class("surface"),
        TuiStyle::new().background(Color::Blue),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert_eq!(cell_colors(&terminal, 0, 1, 12).1, Color::Blue);
    assert_eq!(cell_colors(&terminal, 5, 1, 12).1, Color::Blue);

    Ok(())
}

/// Verifies parent backgrounds fill the top row after scrolling up.
///
/// # Example Under Test
///
/// ```text
/// column([text("Top"), button("Launch"), text("Tail")]).surface
/// PageDown, Up
/// ```
///
/// # Assertions
///
/// - The initial styled render succeeds.
/// - PageDown and Up are handled by the view.
/// - The second styled render succeeds.
/// - Empty and occupied cells on the top row keep the parent background.
#[test]
fn overflowing_column_keeps_parent_background_on_top_row_after_scrolling_up() -> Result<()> {
    let backend = TestBackend::new(12, 2);
    let mut terminal = Terminal::new(backend)?;
    let mut view = column((text("Top"), button("Launch"), text("Tail"))).with_classes("surface");
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::class("surface"),
        TuiStyle::new().background(Color::Blue),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert_eq!(cell_colors(&terminal, 0, 0, 12).1, Color::Blue);
    assert_eq!(cell_colors(&terminal, 5, 0, 12).1, Color::Blue);

    Ok(())
}

/// Verifies row minimum height uses split child widths for wrapped text.
///
/// # Example Under Test
///
/// ```text
/// row([text("Hello World"), text("Side")])
/// terminal width = 12
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The row minimum height accounts for wrapping inside the split child area.
#[test]
fn row_min_height_uses_split_child_widths_for_wrapped_text() -> Result<()> {
    let backend = TestBackend::new(12, 4);
    let mut terminal = Terminal::new(backend)?;
    let view = row(vec![text("Hello World"), text("Side")]);
    let mut min_height = 0;

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        min_height = view.__min_height(&mut ctx);
    })?;

    assert_eq!(min_height, 2);

    Ok(())
}

/// Verifies text-area minimum height counts trailing newline rows.
///
/// # Example Under Test
///
/// ```text
/// text_area("Ada\n")
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The text area minimum height includes the trailing blank line and border.
#[test]
fn text_area_min_height_counts_trailing_newline() -> Result<()> {
    let backend = TestBackend::new(8, 5);
    let mut terminal = Terminal::new(backend)?;
    let view = text_area("Ada\n");
    let mut min_height = 0;

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        min_height = view.__min_height(&mut ctx);
    })?;

    assert_eq!(min_height, 4);

    Ok(())
}

/// Verifies component boundaries backed by [`View`] report wrapped view height.
///
/// # Example Under Test
///
/// ```text
/// component(column([text("One"), text("Two"), text("Three")])).__min_height(ctx)
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The component boundary reports the wrapped column's three-row height.
///
/// # Why
///
/// Parent layouts use component minimum heights to decide whether children need
/// fixed height or overflow scrolling.
#[test]
fn component_view_min_height_uses_wrapped_view_height() -> Result<()> {
    let backend = TestBackend::new(12, 4);
    let mut terminal = Terminal::new(backend)?;
    let view = component(column([text("One"), text("Two"), text("Three")]));
    let mut min_height = 0;

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        min_height = view.__min_height(&mut ctx);
    })?;

    assert_eq!(min_height, 3);

    Ok(())
}

/// Verifies overflowing columns scroll rows produced by wrapped text.
///
/// # Example Under Test
///
/// ```text
/// column([text("Hello World"), text("Bottom")])
/// terminal size = 7x2
/// PageDown
/// ```
///
/// # Assertions
///
/// - The initial render succeeds and hides the later child.
/// - PageDown is handled by the view.
/// - The second render succeeds.
/// - The wrapped text row and later child become visible after scrolling.
#[test]
fn overflowing_column_scrolls_wrapped_text_rows() -> Result<()> {
    let backend = TestBackend::new(7, 2);
    let mut terminal = Terminal::new(backend)?;
    let mut view = column(vec![text("Hello World"), text("Bottom")]);
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert!(symbol_position_opt(&terminal, "B", 7).is_none());

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(symbol_position(&terminal, "W", 7).1, 0);
    assert_eq!(symbol_position(&terminal, "B", 7).1, 1);

    Ok(())
}

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

/// Verifies media direction gives stacked bordered buttons enough height.
///
/// # Example Under Test
///
/// ```text
/// row([button("A"), button("B")]).stack
/// @media (max-width: 12) { .stack { direction: Column } }
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
    let view = row(vec![button("A"), button("B")]).with_classes("stack");
    let stylesheet = Stylesheet::new().media_rule(
        MediaQuery::max_width(12),
        StyleSelector::class("stack"),
        TuiStyle::new().direction(LayoutDirection::Column),
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
/// column([text("Top"), row(buttons).stack, text("End")])
/// @media (max-width: 12) { .stack { direction: Column } }
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
    let view = column((
        text("Top"),
        row(vec![button("A"), button("B"), button("C"), button("D")]).with_classes("stack"),
        text("End"),
    ));
    let stylesheet = Stylesheet::new().media_rule(
        MediaQuery::max_width(12),
        StyleSelector::class("stack"),
        TuiStyle::new().direction(LayoutDirection::Column),
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

/// Verifies overflowing columns can scroll to later children by default.
///
/// # Example Under Test
///
/// ```text
/// column([text rows..., row([button("Launch"), button("Quit")]).focus-actions])
/// PageDown
/// ```
///
/// # Assertions
///
/// - The initial styled render succeeds and hides the later button.
/// - PageDown is handled by the view.
/// - The second styled render succeeds and shows the later button.
#[test]
fn overflowing_column_scrolls_to_later_children_by_default() -> Result<()> {
    let backend = TestBackend::new(12, 6);
    let mut terminal = Terminal::new(backend)?;
    let mut view = column((
        text("One"),
        text("Two"),
        text("Three"),
        text("Four"),
        row(vec![button("Launch"), button("Quit")]).with_classes("focus-actions"),
    ));
    let stylesheet = Stylesheet::new().media_rule(
        MediaQuery::max_width(12),
        StyleSelector::class("focus-actions"),
        TuiStyle::new().direction(LayoutDirection::Column),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert!(symbol_position_opt(&terminal, "Q", 12).is_none());

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert_eq!(symbol_position(&terminal, "Q", 12).1, 4);

    Ok(())
}

/// Verifies page scrolling handles stacked buttons without nested scrolling.
///
/// # Example Under Test
///
/// ```text
/// column([block(text("Top")), row(buttons).stack, text("End")])
/// PageDown, PageDown
/// ```
///
/// # Assertions
///
/// - The first render shows the top block and hides the last button.
/// - The first PageDown scrolls the parent page and reveals middle buttons.
/// - The second PageDown keeps the top block hidden and reveals the last button.
///
/// # Why
///
/// Parent overflow should manage page scrolling when nested stacked content is
/// taller than the viewport.
#[test]
fn overflowing_page_scrolls_stacked_buttons_without_nested_scroll() -> Result<()> {
    let backend = TestBackend::new(12, 6);
    let mut terminal = Terminal::new(backend)?;
    let mut view = column((
        block(text("Top")),
        row(vec![button("A"), button("B"), button("C"), button("D")]).with_classes("stack"),
        text("End"),
    ));
    let stylesheet = Stylesheet::new().media_rule(
        MediaQuery::max_width(12),
        StyleSelector::class("stack"),
        TuiStyle::new().direction(LayoutDirection::Column),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert!(symbol_position_opt(&terminal, "T", 12).is_some());
    assert!(symbol_position_opt(&terminal, "D", 12).is_none());

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert!(symbol_position_opt(&terminal, "T", 12).is_none());
    assert!(symbol_position_opt(&terminal, "C", 12).is_some());

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert!(symbol_position_opt(&terminal, "T", 12).is_none());
    assert!(symbol_position_opt(&terminal, "D", 12).is_some());

    Ok(())
}

/// Verifies row layout stays horizontal without a direction override.
///
/// # Example Under Test
///
/// ```text
/// row([text("A"), text("B")])
/// terminal width = 4
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The view render call succeeds.
/// - The child text views render on the same row in separate columns.
#[test]
fn row_layout_stays_horizontal_without_direction_override() -> Result<()> {
    let backend = TestBackend::new(4, 2);
    let mut terminal = Terminal::new(backend)?;
    let view = row(vec![text("A"), text("B")]);
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(symbol_position(&terminal, "A", 4), (0, 0));
    assert_eq!(symbol_position(&terminal, "B", 4), (2, 0));

    Ok(())
}
