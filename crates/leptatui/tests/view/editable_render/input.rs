/// Verifies input views render their controlled value.
///
/// # Example Under Test
///
/// ```text
/// input("Ada")
/// width = 8, height = 3
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The input renders a default border.
/// - The inner cells contain `A`, `d`, and `a`.
#[test]
fn renders_input_value() -> leptatui::app::Result<()> {
    let backend = TestBackend::new(8, 3);
    let mut terminal = Terminal::new(backend)?;
    let view = input("Ada");

    draw_view(&mut terminal, &view)?;

    assert_eq!(
        cell_symbol(&terminal, 0, 0, 8),
        symbol_border::PLAIN.top_left
    );
    assert_eq!(cell_symbol(&terminal, 1, 1, 8), "A");
    assert_eq!(cell_symbol(&terminal, 2, 1, 8), "d");
    assert_eq!(cell_symbol(&terminal, 3, 1, 8), "a");

    Ok(())
}

/// Verifies input default borders can be disabled through styles.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_inline_style(TuiStyle::new().borders(Borders::NONE))
/// width = 8
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The value starts in the first cell when borders are disabled.
#[test]
fn input_borders_none_disables_default_border() -> leptatui::app::Result<()> {
    let backend = TestBackend::new(8, 1);
    let mut terminal = Terminal::new(backend)?;
    let view = input("Ada").with_inline_style(TuiStyle::new().borders(Borders::NONE));

    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 0, 0, 8), "A");
    assert_eq!(cell_symbol(&terminal, 3, 0, 8), " ");

    Ok(())
}

/// Verifies empty input views render placeholder text.
///
/// # Example Under Test
///
/// ```text
/// input("").placeholder("Name")
/// width = 8, height = 3
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The inner cells contain the first and last placeholder characters.
#[test]
fn renders_input_placeholder_when_value_is_empty() -> leptatui::app::Result<()> {
    let backend = TestBackend::new(8, 3);
    let mut terminal = Terminal::new(backend)?;
    let view = input("").placeholder("Name");

    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 1, 1, 8), "N");
    assert_eq!(cell_symbol(&terminal, 4, 1, 8), "e");

    Ok(())
}

/// Verifies focused input views receive focus stylesheet rules.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true)
/// :focus { fg: Black, bg: Yellow }
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The component render call succeeds.
/// - The focused input cell uses the stylesheet foreground color.
/// - The focused input cell uses the stylesheet background color.
#[test]
fn renders_focused_input_with_focus_stylesheet_rule() -> leptatui::app::Result<()> {
    let backend = TestBackend::new(8, 3);
    let mut terminal = Terminal::new(backend)?;
    let view = input("Ada").with_focus(true);
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

/// Verifies focused inputs place the terminal cursor at the retained cursor.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true)
/// cursor = end
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The focused input sets the terminal cursor after the rendered value.
#[test]
fn focused_input_sets_terminal_cursor_position() -> leptatui::app::Result<()> {
    let backend = TestBackend::new(8, 3);
    let mut terminal = Terminal::new(backend)?;
    let view = input("Ada").with_focus(true);

    draw_view(&mut terminal, &view)?;

    terminal.backend_mut().assert_cursor_position((4, 1));

    Ok(())
}

/// Verifies a cursor outside retained clip geometry is not forwarded.
///
/// # Example Under Test
///
/// ```text
/// 4x1 hidden-overflow parent
/// focused 4x3 input
/// ```
///
/// # Assertions
///
/// - Only the input's top border is visible.
/// - The content-row cursor remains clipped instead of moving below the
///   parent's viewport.
#[test]
fn focused_input_cursor_respects_retained_clip() -> leptatui::app::Result<()> {
    let child = input("A").with_focus(true).with_inline_style(
        TuiStyle::new()
            .box_sizing(BoxSizing::BorderBox)
            .size(LayoutSize::new(
                Dimension::from(Length::cells(4.0)),
                Dimension::from(Length::cells(3.0)),
            )),
    );
    let view = div([child]).with_inline_style(
        TuiStyle::new()
            .size(LayoutSize::new(
                Dimension::from(Length::cells(4.0)),
                Dimension::from(Length::cells(1.0)),
            ))
            .overflow(Axes::all(Overflow::Hidden)),
    );
    let mut terminal = Terminal::new(TestBackend::new(4, 3))?;

    draw_view(&mut terminal, &view)?;

    assert_eq!(
        cell_symbol(&terminal, 0, 0, 4),
        symbol_border::PLAIN.top_left
    );
    terminal.backend_mut().assert_cursor_position((0, 0));
    Ok(())
}

/// Verifies component-backed roots expose focused editable control mode.
///
/// # Example Under Test
///
/// ```text
/// focused Input: Normal, Insert, pending jk exit, Visual
/// focused TextArea: Normal, Insert, pending jk exit, VisualLine
/// focused Button and unfocused Input
/// ```
///
/// # Assertions
///
/// - Focused inputs and text areas report their normal, insert, and visual state.
/// - A pending `jk` sequence reports normal mode before the controlled value updates.
/// - A focused button reports button state.
/// - An unfocused input reports no focused control.
#[test]
fn app_root_reports_focused_editable_control_mode() -> leptatui::app::Result<()> {
    let normal_input = input("Ada").with_focus(true);
    assert_eq!(
        leptatui::AppRoot::__focused_control(&normal_input),
        Some(FocusedControl::Input {
            insert_mode: false,
            visual_mode: false,
        })
    );

    let mut insert_input = input("Ada").with_focus(true);
    editable_state_mut(&mut insert_input).set_mode(VimMode::Insert);
    assert_eq!(
        leptatui::AppRoot::__focused_control(&insert_input),
        Some(FocusedControl::Input {
            insert_mode: true,
            visual_mode: false,
        })
    );

    insert_input.handle_key_event(key_event(KeyCode::Char('j')))?;
    assert_eq!(
        leptatui::AppRoot::__focused_control(&insert_input),
        Some(FocusedControl::Input {
            insert_mode: false,
            visual_mode: false,
        })
    );

    let mut visual_input = input("Ada").with_focus(true);
    editable_state_mut(&mut visual_input).set_mode(VimMode::Visual);
    editable_state_mut(&mut visual_input).set_selection_anchor(Some(0));
    assert_eq!(
        leptatui::AppRoot::__focused_control(&visual_input),
        Some(FocusedControl::Input {
            insert_mode: false,
            visual_mode: true,
        })
    );

    let normal_text_area = text_area("Ada").with_focus(true);
    assert_eq!(
        leptatui::AppRoot::__focused_control(&normal_text_area),
        Some(FocusedControl::TextArea {
            insert_mode: false,
            visual_mode: false,
        })
    );

    let mut insert_text_area = text_area("Ada").with_focus(true);
    editable_state_mut(&mut insert_text_area).set_mode(VimMode::Insert);
    assert_eq!(
        leptatui::AppRoot::__focused_control(&insert_text_area),
        Some(FocusedControl::TextArea {
            insert_mode: true,
            visual_mode: false,
        })
    );

    insert_text_area.handle_key_event(key_event(KeyCode::Char('j')))?;
    assert_eq!(
        leptatui::AppRoot::__focused_control(&insert_text_area),
        Some(FocusedControl::TextArea {
            insert_mode: false,
            visual_mode: false,
        })
    );

    let mut visual_text_area = text_area("Ada").with_focus(true);
    editable_state_mut(&mut visual_text_area).set_mode(VimMode::VisualLine);
    editable_state_mut(&mut visual_text_area).set_selection_anchor(Some(0));
    assert_eq!(
        leptatui::AppRoot::__focused_control(&visual_text_area),
        Some(FocusedControl::TextArea {
            insert_mode: false,
            visual_mode: true,
        })
    );

    assert_eq!(
        leptatui::AppRoot::__focused_control(&button("Save").with_focus(true)),
        Some(FocusedControl::Button)
    );
    assert_eq!(leptatui::AppRoot::__focused_control(&input("Ada")), None);

    Ok(())
}

/// Verifies input rendering clips content around the retained cursor.
///
/// # Example Under Test
///
/// ```text
/// input("abcdef").with_focus(true)
/// width = 4, height = 3
/// cursor = end, then cursor = 0
/// ```
///
/// # Assertions
///
/// - The first render succeeds and shows the tail of the value.
/// - Moving the cursor to the start succeeds.
/// - The second render succeeds and shows the head of the value.
#[test]
fn input_rendering_clips_and_scrolls_around_cursor() -> leptatui::app::Result<()> {
    let backend = TestBackend::new(4, 3);
    let mut terminal = Terminal::new(backend)?;
    let mut view = input("abcdef").with_focus(true);

    draw_view(&mut terminal, &view)?;
    assert_eq!(cell_symbol(&terminal, 1, 1, 4), "e");
    assert_eq!(cell_symbol(&terminal, 2, 1, 4), "f");

    editable_state_mut(&mut view).set_cursor(0);
    draw_view(&mut terminal, &view)?;
    assert_eq!(cell_symbol(&terminal, 1, 1, 4), "a");
    assert_eq!(cell_symbol(&terminal, 2, 1, 4), "b");

    Ok(())
}

/// Verifies visual-mode input selections render selected cells in reverse video.
///
/// # Example Under Test
///
/// ```text
/// input("abcd").with_focus(true)
/// mode = Visual, selection = bytes 1..=2
/// ```
///
/// # Assertions
///
/// - Cells inside the selected byte range use reverse-video styling.
/// - Cells outside the selected byte range do not use reverse-video styling.
#[test]
fn input_visual_selection_renders_reversed_cells() -> leptatui::app::Result<()> {
    let backend = TestBackend::new(8, 3);
    let mut terminal = Terminal::new(backend)?;
    let mut view = input("abcd").with_focus(true);
    editable_state_mut(&mut view).set_mode(VimMode::Visual);
    editable_state_mut(&mut view).set_selection_anchor(Some(1));
    editable_state_mut(&mut view).set_cursor(2);

    draw_view(&mut terminal, &view)?;

    assert!(!cell_modifiers(&terminal, 1, 1, 8).contains(Modifier::REVERSED));
    assert!(cell_modifiers(&terminal, 2, 1, 8).contains(Modifier::REVERSED));
    assert!(cell_modifiers(&terminal, 3, 1, 8).contains(Modifier::REVERSED));
    assert!(!cell_modifiers(&terminal, 4, 1, 8).contains(Modifier::REVERSED));

    Ok(())
}

/// Verifies a pending insert-mode `j` renders as a reversed preview character.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true)
/// mode = Insert, key = j
/// ```
///
/// # Assertions
///
/// - The pending key is handled without committing the controlled value.
/// - The preview character renders in reverse video.
/// - The terminal cursor remains on the preview character.
#[test]
fn input_pending_insert_j_renders_reversed_preview() -> leptatui::app::Result<()> {
    let backend = TestBackend::new(8, 3);
    let mut terminal = Terminal::new(backend)?;
    let mut view = input("Ada").with_focus(true);
    editable_state_mut(&mut view).set_mode(VimMode::Insert);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('j')))?,
        KeyControl::Handled
    );
    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 4, 1, 8), "j");
    assert!(cell_modifiers(&terminal, 4, 1, 8).contains(Modifier::REVERSED));
    terminal.backend_mut().assert_cursor_position((4, 1));

    Ok(())
}

/// Verifies an expired pending insert-mode `j` renders without preview styling.
///
/// # Example Under Test
///
/// ```text
/// input("Ada").with_focus(true)
/// mode = Insert, key = j, elapsed = 1100 ms
/// ```
///
/// # Assertions
///
/// - The expired pending character remains visible without reverse video.
/// - The terminal cursor advances past the expired character.
/// - The focused input remains in insert mode.
#[test]
fn input_pending_insert_j_preview_expires_to_insert_cursor() -> leptatui::app::Result<()> {
    let backend = TestBackend::new(8, 3);
    let mut terminal = Terminal::new(backend)?;
    let mut view = input("Ada").with_focus(true);
    editable_state_mut(&mut view).set_mode(VimMode::Insert);

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Char('j')))?,
        KeyControl::Handled
    );
    thread::sleep(Duration::from_millis(1100));
    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 4, 1, 8), "j");
    assert!(!cell_modifiers(&terminal, 4, 1, 8).contains(Modifier::REVERSED));
    terminal.backend_mut().assert_cursor_position((5, 1));
    assert_eq!(
        leptatui::AppRoot::__focused_control(&view),
        Some(FocusedControl::Input {
            insert_mode: true,
            visual_mode: false,
        })
    );

    Ok(())
}
