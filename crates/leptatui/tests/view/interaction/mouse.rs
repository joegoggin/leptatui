/// Custom view recording whether its event hook receives mouse input.
struct MouseEventProbe {
    /// Whether the custom event hook observed a mouse event.
    seen: Rc<Cell<bool>>,
}

/// View that constrains a child to three rows while retaining normal traversal.
struct ClippedFocusPanel {
    /// Child view rendered inside the constrained panel.
    view: AnyView,
}

impl View for ClippedFocusPanel {
    /// Renders the child view in the panel's assigned area.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Rendering context supplied by the parent layout.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        ctx.render_view(&self.view)
    }

    /// Returns the panel's constrained intrinsic geometry.
    ///
    /// # Arguments
    ///
    /// * `known_dimensions` — Exact dimensions supplied by parent layout.
    /// * `available_space` — Soft constraints for unknown dimensions.
    /// * `_ctx` — Rendering context containing styles and inherited state.
    ///
    /// # Returns
    ///
    /// A [`LayoutSize`] with a three-row intrinsic height.
    fn measure(
        &self,
        known_dimensions: LayoutSize<Option<f32>>,
        available_space: LayoutSize<AvailableSpace>,
        _ctx: &mut RenderCtx<'_, '_>,
    ) -> LayoutSize<f32> {
        LayoutSize::new(
            known_dimensions.width.unwrap_or(match available_space.width {
                AvailableSpace::Definite(width) => width,
                AvailableSpace::MinContent | AvailableSpace::MaxContent => 1.0,
            }),
            known_dimensions.height.unwrap_or(3.0),
        )
    }

    /// Returns the child view for default traversal.
    ///
    /// # Returns
    ///
    /// A slice containing the panel's child view.
    fn children(&self) -> &[AnyView] {
        std::slice::from_ref(&self.view)
    }

    /// Returns the mutable child view for default traversal.
    ///
    /// # Returns
    ///
    /// A mutable slice containing the panel's child view.
    fn children_mut(&mut self) -> &mut [AnyView] {
        std::slice::from_mut(&mut self.view)
    }

    /// Returns the panel for concrete-type inspection.
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    /// Returns the mutable panel for concrete-type inspection.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
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
/// div((button("One"), button("Two"))).display(Flex)
/// MouseMoved(6, 1)
/// ```
///
/// # Assertions
///
/// - The first button remains unfocused.
/// - The button beneath the pointer receives focus.
#[test]
fn mouse_move_focuses_button_under_pointer() -> Result<()> {
    let mut terminal = Terminal::new(TestBackend::new(20, 3))?;
    let mut view = div([button("One"), button("Two")])
        .with_inline_style(TuiStyle::new().display(Display::Flex));

    draw_view(&mut terminal, &view)?;
    view.handle_event(mouse(MouseEventKind::Moved, 6, 1))?;

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
/// div(("one", "two", "three")), viewport height = 2
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
    let mut view = div([text("one"), text("two"), text("three")]);

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
/// div((component(overflowing inner Div), button("Visible")))
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
        view: div([
            text("one"),
            text("two"),
            text("three"),
            text("four"),
            text("five"),
            text("six"),
        ])
        .into_view(),
    });
    let mut view = div((inner, button("Visible")));

    draw_view(&mut terminal, &view)?;
    assert!(rendered_text(&terminal).contains("one"));

    for _ in 0..6 {
        view.handle_event(mouse(MouseEventKind::ScrollDown, 0, 0))?;
    }
    draw_view(&mut terminal, &view)?;

    assert!(rendered_text(&terminal).contains("Visible"));

    Ok(())
}

/// Verifies nested clipped views compose mouse hit-test coordinates.
///
/// # Example Under Test
///
/// ```text
/// div(("Header", clipped panel(scrolled div((button("Nested"), button("Later"))))))
/// MouseMoved(1, 0), MouseMoved(1, 1)
/// ```
///
/// # Assertions
///
/// - Moving over the outer header does not focus the nested clipped button.
/// - Moving over the visible nested button focuses that button.
///
/// # Why
///
/// Each offscreen render must retain its parent's clipping translation so
/// nested controls record terminal coordinates instead of buffer-local rows.
#[test]
fn nested_clipped_views_compose_mouse_hit_coordinates() -> Result<()> {
    let mut inner_terminal = Terminal::new(TestBackend::new(12, 3))?;
    let mut inner = div([button("Nested"), button("Later")]);
    draw_view(&mut inner_terminal, &inner)?;
    inner.handle_key_event(key_event(KeyCode::Down))?;

    let mut terminal = Terminal::new(TestBackend::new(12, 3))?;
    let panel = ClippedFocusPanel {
        view: inner.into_view(),
    };
    let mut view = div((text("Header"), panel));
    draw_view(&mut terminal, &view)?;

    view.handle_event(mouse(MouseEventKind::Moved, 1, 0))?;
    assert_eq!(button_focuses(&view), vec![false, false]);

    view.handle_event(mouse(MouseEventKind::Moved, 1, 1))?;
    assert_eq!(button_focuses(&view), vec![true, false]);

    Ok(())
}

/// Verifies horizontal clipping maps partially visible child hit coordinates.
///
/// # Example Under Test
///
/// ```text
/// 8x3 flex div([8x3 button("First"), 8x3 button("Second")])
/// overflow: hidden clip
/// ScrollRight x4
/// MouseMoved(1, 1), MouseMoved(7, 1)
/// ```
///
/// # Assertions
///
/// - Moving over the retained right half of the first button focuses it.
/// - Moving over the retained left half of the second button focuses it.
///
/// # Why
///
/// Offscreen buffers must include the horizontal source offset when mapping
/// clipped child hit areas back to terminal coordinates.
#[test]
fn horizontal_clipping_maps_partial_child_hit_coordinates() -> Result<()> {
    let child_style = TuiStyle::new()
        .size(LayoutSize::new(
            Dimension::from(Length::cells(8.0)),
            Dimension::from(Length::cells(3.0)),
        ))
        .flex_shrink(0.0);
    let mut view = div([
        button("First").with_inline_style(child_style),
        button("Second").with_inline_style(child_style),
    ])
    .with_inline_style(
        TuiStyle::new()
            .display(Display::Flex)
            .size(LayoutSize::new(
                Dimension::from(Length::cells(8.0)),
                Dimension::from(Length::cells(3.0)),
            ))
            .overflow(Axes::new(Overflow::Hidden, Overflow::Clip)),
    );
    let mut terminal = Terminal::new(TestBackend::new(8, 3))?;

    draw_view(&mut terminal, &view)?;
    for _ in 0..4 {
        view.handle_event(mouse(MouseEventKind::ScrollRight, 1, 1))?;
    }
    view.__clear_hit_areas();
    draw_view(&mut terminal, &view)?;

    view.handle_event(mouse(MouseEventKind::Moved, 1, 1))?;
    assert_eq!(button_focuses(&view), vec![true, false]);

    view.handle_event(mouse(MouseEventKind::Moved, 7, 1))?;
    assert_eq!(
        button_focuses(&view),
        vec![false, true],
        "rendered text: {:?}",
        rendered_text(&terminal)
    );
    Ok(())
}

/// Verifies concrete app roots clear hit areas for controls scrolled offscreen.
///
/// # Example Under Test
///
/// ```text
/// div((button("Hidden"), button("Visible")))
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
    let mut view = div((button("Hidden"), button("Visible")));
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
