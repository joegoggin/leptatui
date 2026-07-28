/// Verifies activation keys do not activate focused editable controls.
///
/// # Example Under Test
///
/// ```text
/// div([Input, button("Submit")])
/// Tab, Enter, Space, Tab, Enter
/// ```
///
/// # Assertions
///
/// - The editable input receives focus.
/// - Enter and space return [`AppControl::Continue`] without running callbacks
///   while the editable input is focused.
/// - The button receives focus after another tab event.
/// - Enter returns [`AppControl::Continue`] and runs the focused button callback.
#[test]
fn enter_and_space_do_not_activate_focused_editable_controls() -> leptatui::app::Result<()> {
    let count = Rc::new(Cell::new(0));
    let submit_count = Rc::clone(&count);
    let mut view = div((
        editable_input("Ada"),
        button("Submit").on_press(move || {
            submit_count.set(submit_count.get() + 1);
            AppControl::Continue
        }),
    ));

    view.handle_event(key(KeyCode::Tab))?;
    assert_eq!(control_focuses(&view), vec![true, false]);

    assert_eq!(
        view.handle_event(key(KeyCode::Enter))?,
        AppControl::Continue
    );
    assert_eq!(
        view.handle_event(key(KeyCode::Char(' ')))?,
        AppControl::Continue
    );
    assert_eq!(count.get(), 0);

    view.handle_event(key(KeyCode::Tab))?;
    assert_eq!(control_focuses(&view), vec![false, true]);
    assert_eq!(
        view.handle_event(key(KeyCode::Enter))?,
        AppControl::Continue
    );
    assert_eq!(count.get(), 1);

    Ok(())
}

/// Verifies focused button actions can request app exit.
///
/// # Example Under Test
///
/// ```text
/// button("Quit").on_press(|| AppControl::Exit)
/// Tab, Enter
/// ```
///
/// # Assertions
///
/// - The tab event succeeds and focuses the button.
/// - The enter event returns [`AppControl::Exit`].
#[test]
fn focused_button_action_can_exit_app_loop() -> leptatui::app::Result<()> {
    let mut view = button("Quit").on_press(|| AppControl::Exit);

    view.handle_event(key(KeyCode::Tab))?;

    assert_eq!(view.handle_event(key(KeyCode::Enter))?, AppControl::Exit);

    Ok(())
}

/// Verifies focused buttons render with focus stylesheet rules.
///
/// # Example Under Test
///
/// ```text
/// div([button("One"), button("Two")])
/// Stylesheet::new().rule(StyleSelector::focus(), black on yellow)
/// with_focus(true)
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The view render call succeeds.
/// - The rendered focused button label exists.
/// - The focused cell has a black foreground.
/// - The focused cell has a yellow background.
///
/// # Why
///
/// Focus selector state should affect rendered button styling.
#[test]
fn renders_focused_button_with_focus_stylesheet_rule() -> leptatui::app::Result<()> {
    let backend = TestBackend::new(24, 5);
    let mut terminal = Terminal::new(backend)?;
    let view = div([button("One").with_focus(true), button("Two")])
        .with_inline_style(TuiStyle::new().display(Display::Flex));
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

    let focused_cell = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .find(|cell| cell.symbol() == "O")
        .expect("rendered focused button label");

    assert_eq!(focused_cell.fg, Color::Black);
    assert_eq!(focused_cell.bg, Color::Yellow);

    Ok(())
}
