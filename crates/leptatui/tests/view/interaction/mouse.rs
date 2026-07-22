/// Verifies moving the mouse over a button focuses it.
#[test]
fn mouse_move_focuses_button_under_pointer() -> Result<()> {
    let mut terminal = Terminal::new(TestBackend::new(20, 3))?;
    let mut view = row([button("One"), button("Two")]);

    draw_view(&mut terminal, &view)?;
    view.handle_event(mouse(MouseEventKind::Moved, 12, 1))?;

    assert_eq!(button_focuses(&view), vec![false, true]);
    Ok(())
}

/// Verifies left-clicking a button invokes its action.
#[test]
fn mouse_click_activates_button_under_pointer() -> Result<()> {
    let mut terminal = Terminal::new(TestBackend::new(12, 3))?;
    let count = Rc::new(Cell::new(0));
    let count_for_button = Rc::clone(&count);
    let mut view = button("Run").on_press(move || {
        count_for_button.set(count_for_button.get() + 1);
        AppControl::Continue
    });

    draw_view(&mut terminal, &view)?;
    assert_eq!(
        view.handle_event(mouse(MouseEventKind::Down(MouseButton::Left), 1, 1))?,
        AppControl::Continue
    );

    assert_eq!(count.get(), 1);
    assert_eq!(button_focuses(&view), vec![true]);
    Ok(())
}

/// Verifies mouse wheel events scroll overflowing content under the pointer.
#[test]
fn mouse_wheel_scrolls_overflowing_column_under_pointer() -> Result<()> {
    let mut terminal = Terminal::new(TestBackend::new(12, 2))?;
    let mut view = column([text("one"), text("two"), text("three")]);

    draw_view(&mut terminal, &view)?;
    assert_eq!(scroll_offset(&view), 0);

    view.handle_event(mouse(MouseEventKind::ScrollDown, 0, 0))?;
    assert_eq!(scroll_offset(&view), 1);

    draw_view(&mut terminal, &view)?;
    view.handle_event(mouse(MouseEventKind::ScrollUp, 0, 0))?;
    assert_eq!(scroll_offset(&view), 0);
    Ok(())
}

/// Verifies wheel scrolling moves to a parent after an inner boundary.
#[test]
fn mouse_wheel_bubbles_from_inner_scroll_boundary_to_parent() -> Result<()> {
    let mut terminal = Terminal::new(TestBackend::new(12, 3))?;
    let inner = component(ConstrainedScrollPanel {
        view: column([
            text("one"),
            text("two"),
            text("three"),
            text("four"),
            text("five"),
            text("six"),
        ])
        .into_view(),
    });
    let mut view = column((inner, button("Visible")));

    draw_view(&mut terminal, &view)?;
    assert!(rendered_text(&terminal).contains("one"));

    for _ in 0..6 {
        view.handle_event(mouse(MouseEventKind::ScrollDown, 0, 0))?;
    }
    draw_view(&mut terminal, &view)?;

    assert!(rendered_text(&terminal).contains("Visible"));

    Ok(())
}

/// Verifies moving over an inline Markdown link focuses it.
#[test]
fn mouse_move_focuses_inline_markdown_link_under_pointer() -> Result<()> {
    let mut terminal = Terminal::new(TestBackend::new(20, 3))?;
    let mut view = markdown("[Docs](https://example.com)");

    draw_view(&mut terminal, view.as_view())?;
    view.handle_event(mouse(MouseEventKind::Moved, 0, 0))?;

    assert_eq!(view.__focused_control(), Some(FocusedControl::Link));
    Ok(())
}

/// Verifies mouse hit testing follows Markdown word wrapping.
#[test]
fn mouse_move_focuses_word_wrapped_markdown_link() -> Result<()> {
    let mut terminal = Terminal::new(TestBackend::new(10, 2))?;
    let mut view = markdown("123456 [Link](https://example.com)");

    draw_view(&mut terminal, view.as_view())?;
    assert_eq!(symbol_position(&terminal, "L", 10), (0, 1));
    view.handle_event(mouse(MouseEventKind::Moved, 1, 1))?;

    assert_eq!(view.__focused_control(), Some(FocusedControl::Link));

    Ok(())
}
