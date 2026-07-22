/// Custom view recording whether its event hook receives mouse input.
struct MouseEventProbe {
    /// Whether the custom event hook observed a mouse event.
    seen: Rc<Cell<bool>>,
}

impl View for MouseEventProbe {
    /// Renders the behavior-only probe without terminal output.
    fn render(&self, _ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        Ok(())
    }

    /// Returns the probe for concrete-type inspection.
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    /// Returns the mutable probe for concrete-type inspection.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// Records mouse events received through normal custom event dispatch.
    fn on_event(&mut self, event: &Event) -> Result<AppControl> {
        if matches!(event, Event::Mouse(_)) {
            self.seen.set(true);
        }
        Ok(AppControl::Continue)
    }
}

/// Verifies mouse events reach custom view hooks across component boundaries.
///
/// # Example Under Test
///
/// ```text
/// MouseEventProbe
/// Component(MouseEventProbe)
/// MouseMoved(1, 1)
/// ```
///
/// # Assertions
///
/// - A direct custom view receives the mouse event through `on_event`.
/// - A custom view inside a component boundary receives the same mouse event.
#[test]
fn mouse_events_reach_custom_view_hooks() -> Result<()> {
    let direct_seen = Rc::new(Cell::new(false));
    let mut direct = MouseEventProbe {
        seen: Rc::clone(&direct_seen),
    };
    direct.handle_event(mouse(MouseEventKind::Moved, 1, 1))?;
    assert!(direct_seen.get());

    let nested_seen = Rc::new(Cell::new(false));
    let mut nested = component(MouseEventProbe {
        seen: Rc::clone(&nested_seen),
    });
    nested.handle_event(mouse(MouseEventKind::Moved, 1, 1))?;
    assert!(nested_seen.get());

    Ok(())
}

/// Verifies moving the mouse over a button focuses it.
///
/// # Example Under Test
///
/// ```text
/// Row(Button("One"), Button("Two"))
/// MouseMoved(12, 1)
/// ```
///
/// # Assertions
///
/// - The first button remains unfocused.
/// - The button beneath the pointer receives focus.
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
///
/// # Example Under Test
///
/// ```text
/// Button("Run", on_press = increment_count)
/// LeftButtonDown(1, 1)
/// ```
///
/// # Assertions
///
/// - The click is handled without exiting the application.
/// - The button action runs exactly once and the button receives focus.
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
///
/// # Example Under Test
///
/// ```text
/// Column("one", "two", "three"), viewport height = 2
/// ScrollDown, ScrollUp
/// ```
///
/// # Assertions
///
/// - Scrolling down advances the column offset by one row.
/// - Scrolling up restores the original offset.
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
///
/// # Example Under Test
///
/// ```text
/// Column(Component(overflowing inner column), Button("Visible"))
/// repeated ScrollDown at (0, 0)
/// ```
///
/// # Assertions
///
/// - The inner component initially displays its first row.
/// - Further wheel input bubbles to the parent after the inner scroll boundary.
/// - The parent scroll reveals the trailing button.
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

/// Verifies concrete app roots clear hit areas for controls scrolled offscreen.
///
/// # Example Under Test
///
/// ```text
/// Column(Button("Hidden"), Button("Visible"))
/// PageDown
/// MouseMoved(1, 1)
/// ```
///
/// # Assertions
///
/// - The initial app-root render displays the first button.
/// - Scrolling and redrawing replaces it with the second button.
/// - Pointer movement focuses the visible button instead of the hidden button.
#[test]
fn concrete_app_root_clears_offscreen_control_hit_areas() -> Result<()> {
    let mut terminal = Terminal::new(TestBackend::new(12, 3))?;
    let mut view = column((button("Hidden"), button("Visible")));
    let mut first_render_result = Ok(());

    terminal.draw(|frame| {
        first_render_result = leptatui::AppRoot::render(&mut view, frame);
    })?;
    first_render_result?;
    assert!(rendered_text(&terminal).contains("Hidden"));

    view.handle_event(key(KeyCode::PageDown))?;
    let mut second_render_result = Ok(());
    terminal.draw(|frame| {
        second_render_result = leptatui::AppRoot::render(&mut view, frame);
    })?;
    second_render_result?;
    let rendered = rendered_text(&terminal);
    assert!(!rendered.contains("Hidden"), "rendered text: {rendered:?}");
    assert!(rendered.contains("Visible"), "rendered text: {rendered:?}");

    view.handle_event(mouse(MouseEventKind::Moved, 1, 1))?;
    assert_eq!(button_focuses(&view), vec![false, true]);

    Ok(())
}

/// Verifies moving over an inline Markdown link focuses it.
///
/// # Example Under Test
///
/// ```text
/// [Docs](https://example.com)
/// MouseMoved(0, 0)
/// ```
///
/// # Assertions
///
/// - The inline link beneath the pointer becomes the focused control.
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
///
/// # Example Under Test
///
/// ```text
/// 123456 [Link](https://example.com), viewport width = 10
/// MouseMoved(1, 1)
/// ```
///
/// # Assertions
///
/// - The link wraps onto the second terminal row.
/// - Moving over its wrapped segment focuses the link.
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
