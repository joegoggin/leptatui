/// Verifies text-area views render multiline controlled values.
///
/// # Example Under Test
///
/// ```text
/// text_area("One\nTwo")
/// width = 8, height = 4
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The text area renders a default border.
/// - The first line starts on the first inner row.
/// - The second line starts on the second inner row.
#[test]
fn renders_text_area_multiline_value() -> Result<()> {
    let backend = TestBackend::new(8, 4);
    let mut terminal = Terminal::new(backend)?;
    let view = text_area("One\nTwo");

    draw_view(&mut terminal, &view)?;

    assert_eq!(
        cell_symbol(&terminal, 0, 0, 8),
        symbol_border::PLAIN.top_left
    );
    assert_eq!(cell_symbol(&terminal, 1, 1, 8), "O");
    assert_eq!(cell_symbol(&terminal, 1, 2, 8), "T");

    Ok(())
}

/// Verifies text-area default borders can be disabled through styles.
///
/// # Example Under Test
///
/// ```text
/// text_area("One\nTwo").with_inline_style(TuiStyle::new().borders(Borders::NONE))
/// width = 8, height = 2
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - Lines start in the first column when borders are disabled.
#[test]
fn text_area_borders_none_disables_default_border() -> Result<()> {
    let backend = TestBackend::new(8, 2);
    let mut terminal = Terminal::new(backend)?;
    let view = text_area("One\nTwo").with_inline_style(TuiStyle::new().borders(Borders::NONE));

    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 0, 0, 8), "O");
    assert_eq!(cell_symbol(&terminal, 0, 1, 8), "T");

    Ok(())
}

/// Verifies empty text areas render placeholder text.
///
/// # Example Under Test
///
/// ```text
/// text_area("").placeholder("Notes")
/// width = 8, height = 3
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The inner cells contain the first and last placeholder characters.
#[test]
fn renders_text_area_placeholder_when_value_is_empty() -> Result<()> {
    let backend = TestBackend::new(8, 3);
    let mut terminal = Terminal::new(backend)?;
    let view = text_area("").placeholder("Notes");

    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 1, 1, 8), "N");
    assert_eq!(cell_symbol(&terminal, 5, 1, 8), "s");

    Ok(())
}

/// Verifies focused text areas receive focus stylesheet rules.
///
/// # Example Under Test
///
/// ```text
/// text_area("Ada").with_focus(true)
/// :focus { fg: Black, bg: Yellow }
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The component render call succeeds.
/// - The focused text-area cell uses the stylesheet foreground color.
/// - The focused text-area cell uses the stylesheet background color.
#[test]
fn renders_focused_text_area_with_focus_stylesheet_rule() -> Result<()> {
    let backend = TestBackend::new(8, 3);
    let mut terminal = Terminal::new(backend)?;
    let view = text_area("Ada").with_focus(true);
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::focus(),
        TuiStyle::new()
            .foreground(Color::Black)
            .background(Color::Yellow),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    let (fg, bg) = cell_colors(&terminal, 1, 1, 8);
    assert_eq!(fg, Color::Black);
    assert_eq!(bg, Color::Yellow);

    Ok(())
}

/// Verifies focused text areas place the terminal cursor at the retained cursor.
///
/// # Example Under Test
///
/// ```text
/// text_area("one\ntwo").with_focus(true)
/// cursor = end
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The focused text area sets the terminal cursor on the final row.
#[test]
fn focused_text_area_sets_terminal_cursor_position() -> Result<()> {
    let backend = TestBackend::new(8, 4);
    let mut terminal = Terminal::new(backend)?;
    let view = text_area("one\ntwo").with_focus(true);

    draw_view(&mut terminal, &view)?;

    terminal.backend_mut().assert_cursor_position((4, 2));

    Ok(())
}

/// Verifies text-area rendering scrolls vertically around the retained cursor.
///
/// # Example Under Test
///
/// ```text
/// text_area("aaa\nbbb\nccc").with_focus(true)
/// height = 4
/// cursor = end, then cursor = 0
/// ```
///
/// # Assertions
///
/// - The first render succeeds and shows the tail of the multiline value.
/// - Moving the cursor to the start succeeds.
/// - The second render succeeds and shows the head of the multiline value.
#[test]
fn text_area_rendering_scrolls_vertically_around_cursor() -> Result<()> {
    let backend = TestBackend::new(8, 4);
    let mut terminal = Terminal::new(backend)?;
    let mut view = text_area("aaa\nbbb\nccc").with_focus(true);

    draw_view(&mut terminal, &view)?;
    assert_eq!(cell_symbol(&terminal, 1, 1, 8), "b");
    assert_eq!(cell_symbol(&terminal, 1, 2, 8), "c");

    editable_state_mut(&mut view).set_cursor(0);
    draw_view(&mut terminal, &view)?;
    assert_eq!(cell_symbol(&terminal, 1, 1, 8), "a");
    assert_eq!(cell_symbol(&terminal, 1, 2, 8), "b");

    Ok(())
}

/// Verifies visual-line text-area selections render selected lines in reverse video.
///
/// # Example Under Test
///
/// ```text
/// text_area("one\ntwo\nthree").with_focus(true)
/// mode = VisualLine, selection = second and third lines
/// ```
///
/// # Assertions
///
/// - The first logical line remains unselected.
/// - The second and third logical lines render in reverse video.
#[test]
fn text_area_visual_line_selection_renders_reversed_cells() -> Result<()> {
    let backend = TestBackend::new(10, 5);
    let mut terminal = Terminal::new(backend)?;
    let mut view = text_area("one\ntwo\nthree").with_focus(true);
    editable_state_mut(&mut view).set_mode(VimMode::VisualLine);
    editable_state_mut(&mut view).set_selection_anchor(Some(4));
    editable_state_mut(&mut view).set_cursor(8);

    draw_view(&mut terminal, &view)?;

    assert!(!cell_modifiers(&terminal, 1, 1, 10).contains(Modifier::REVERSED));
    assert!(cell_modifiers(&terminal, 1, 2, 10).contains(Modifier::REVERSED));
    assert!(cell_modifiers(&terminal, 1, 3, 10).contains(Modifier::REVERSED));

    Ok(())
}

/// Verifies a wrapped pending insert-mode `j` renders where the preview wraps.
///
/// # Example Under Test
///
/// ```text
/// text_area("Ada").with_focus(true)
/// width = 5, mode = Insert, key = j
/// ```
///
/// # Assertions
///
/// - The pending key is handled without committing the controlled value.
/// - The preview character wraps to the next row in reverse video.
/// - The terminal cursor is placed on the wrapped preview character.
#[test]
fn text_area_pending_insert_j_renders_reversed_wrapped_preview() -> Result<()> {
    let backend = TestBackend::new(5, 4);
    let mut terminal = Terminal::new(backend)?;
    let mut view = text_area("Ada").with_focus(true);
    editable_state_mut(&mut view).set_mode(VimMode::Insert);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('j')))?,
        KeyControl::Handled
    );
    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 1, 2, 5), "j");
    assert!(cell_modifiers(&terminal, 1, 2, 5).contains(Modifier::REVERSED));
    terminal.backend_mut().assert_cursor_position((1, 2));

    Ok(())
}

/// Verifies columns reserve multiline text-area render height.
///
/// # Example Under Test
///
/// ```text
/// column([text_area("Hello World"), text("End")])
/// width = 6
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The following text view renders after the wrapped text-area rows.
#[test]
fn column_reserves_height_for_wrapped_text_area() -> Result<()> {
    let backend = TestBackend::new(6, 7);
    let mut terminal = Terminal::new(backend)?;
    let view = column((text_area("Hello World"), text("End")));

    draw_view(&mut terminal, &view)?;

    assert_eq!(symbol_position(&terminal, "E", 6), (0, 6));

    Ok(())
}

/// Verifies columns reserve wrapped text render height.
///
/// # Example Under Test
///
/// ```text
/// column([text("Hello World"), text("End")])
/// width = 6
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The view render call succeeds.
/// - Wrapped text occupies the first two rows.
/// - The following text view renders on the third row.
#[test]
fn column_reserves_height_for_wrapped_text() -> Result<()> {
    let backend = TestBackend::new(6, 3);
    let mut terminal = Terminal::new(backend)?;
    let view = column(vec![text("Hello World"), text("End")]);
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(symbol_position(&terminal, "W", 6).1, 1);
    assert_eq!(symbol_position(&terminal, "E", 6).1, 2);

    Ok(())
}
