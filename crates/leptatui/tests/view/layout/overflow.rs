/// Verifies fitting columns do not render a scrollbar.
///
/// # Example Under Test
///
/// ```text
/// div([text("12345678")])
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
    let view = div([text("12345678")]);
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(cell_symbol(&terminal, 7, 0, 8), "8");

    Ok(())
}

/// Verifies clipped overflow does not create a scroll container.
///
/// # Example Under Test
///
/// ```text
/// div(["12345678", "Two", "Three"])
/// overflow: clip
/// terminal size: 8x2
/// PageDown
/// ```
///
/// # Assertions
///
/// - The first row retains all eight text columns without reserving a scrollbar.
/// - PageDown passes because clipped overflow is not scrollable.
#[test]
fn clipped_overflow_does_not_scroll_or_reserve_a_scrollbar() -> Result<()> {
    let backend = TestBackend::new(8, 2);
    let mut terminal = Terminal::new(backend)?;
    let mut view = div(vec![text("12345678"), text("Two"), text("Three")])
        .with_inline_style(TuiStyle::new().overflow(Axes::all(Overflow::Clip)));
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(cell_symbol(&terminal, 7, 0, 8), "8");
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Pass
    );
    Ok(())
}

/// Verifies visible overflow paints outside its box without scrolling.
///
/// # Example Under Test
///
/// ```text
/// outer div([
///   8x1 div(["One", "Two"]) with overflow: visible
/// ])
/// terminal size: 8x2
/// PageDown
/// ```
///
/// # Assertions
///
/// - The second child paints below the one-row inner container.
/// - The overflow does not render a scrollbar.
/// - PageDown passes because visible overflow is not scrollable.
#[test]
fn visible_overflow_paints_outside_its_box_without_scrolling() -> Result<()> {
    let inner = div(vec![text("One"), text("Two")]).with_inline_style(
        TuiStyle::new()
            .size(LayoutSize::new(
                Dimension::from(Length::cells(8.0)),
                Dimension::from(Length::cells(1.0)),
            ))
            .overflow(Axes::all(Overflow::Visible)),
    );
    let mut view = div([inner]);
    let mut terminal = Terminal::new(TestBackend::new(8, 2))?;

    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 0, 1, 8), "T");
    assert_eq!(cell_symbol(&terminal, 7, 0, 8), " ");
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Pass
    );
    Ok(())
}

/// Verifies hidden overflow scrolls without rendering a scrollbar.
///
/// # Example Under Test
///
/// ```text
/// div(["12345678", "Two", "Three"])
/// overflow: hidden
/// terminal size: 8x2
/// PageDown
/// ```
///
/// # Assertions
///
/// - The first row retains all eight text columns without a scrollbar.
/// - PageDown is handled by the hidden scroll container.
/// - The second row of content moves to the top after scrolling.
#[test]
fn hidden_overflow_scrolls_without_rendering_a_scrollbar() -> Result<()> {
    let backend = TestBackend::new(8, 2);
    let mut terminal = Terminal::new(backend)?;
    let mut view = div(vec![text("12345678"), text("Two"), text("Three")])
        .with_inline_style(TuiStyle::new().overflow(Axes::all(Overflow::Hidden)));

    draw_view(&mut terminal, &view)?;
    assert_eq!(cell_symbol(&terminal, 7, 0, 8), "8");

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 0, 0, 8), "T");
    Ok(())
}

/// Verifies nested auto overflow reserves a scrollbar only when needed.
///
/// # Example Under Test
///
/// ```text
/// 8x1 auto div(["12345678"])
/// 8x1 auto div(["12345678", "Two"])
/// terminal size: 8x2
/// ```
///
/// # Assertions
///
/// - Fitting auto content retains all eight text columns.
/// - Overflowing auto content wraps before the reserved scrollbar column.
/// - The overflowing container renders its scrollbar in the eighth column.
#[test]
fn nested_auto_overflow_conditionally_reserves_a_scrollbar() -> Result<()> {
    let fixed_size = LayoutSize::new(
        Dimension::from(Length::cells(8.0)),
        Dimension::from(Length::cells(1.0)),
    );
    let fitting = div([text("12345678")]).with_inline_style(
        TuiStyle::new()
            .size(fixed_size)
            .overflow(Axes::new(Overflow::Visible, Overflow::Auto)),
    );
    let overflowing = div(vec![text("12345678"), text("Two")]).with_inline_style(
        TuiStyle::new()
            .size(fixed_size)
            .overflow(Axes::new(Overflow::Visible, Overflow::Auto)),
    );
    let view = div((fitting, overflowing));
    let mut terminal = Terminal::new(TestBackend::new(8, 2))?;

    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 7, 0, 8), "8");
    assert_eq!(cell_symbol(&terminal, 6, 1, 8), "7");
    assert_eq!(cell_symbol(&terminal, 7, 1, 8), symbol_block::FULL);
    Ok(())
}

/// Verifies scroll overflow always reserves and renders its scrollbar.
///
/// # Example Under Test
///
/// ```text
/// div(["1234567"])
/// overflow: scroll
/// terminal size: 8x1
/// PageDown
/// ```
///
/// # Assertions
///
/// - The fitting text occupies the first seven columns.
/// - The scrollbar occupies the eighth column.
/// - PageDown passes because no scroll offset is available.
#[test]
fn scroll_overflow_always_reserves_and_renders_scrollbar() -> Result<()> {
    let backend = TestBackend::new(8, 1);
    let mut terminal = Terminal::new(backend)?;
    let mut view = div([text("1234567")])
        .with_inline_style(
            TuiStyle::new().overflow(Axes::new(Overflow::Visible, Overflow::Scroll)),
        );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(cell_symbol(&terminal, 6, 0, 8), "7");
    assert_eq!(cell_symbol(&terminal, 7, 0, 8), symbol_block::FULL);
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Pass
    );
    Ok(())
}

/// Verifies overflowing columns render a right-side scrollbar.
///
/// # Example Under Test
///
/// ```text
/// div([text("One"), text("Two"), text("Three")])
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
    let view = div(vec![text("One"), text("Two"), text("Three")]);
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
/// dynamic(|| div([text("One"), text("Two"), text("Three")]))
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
    let mut view = dynamic(|| div(vec![text("One"), text("Two"), text("Three")]));

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
/// div([text("123456"), text("more"), text("tail")])
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
    let view = div(vec![text("123456"), text("more"), text("tail")]);
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
/// div([text("One"), text("Two"), text("Three")])
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
    let mut view = div(vec![text("One"), text("Two"), text("Three")]);
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
/// div(Line 0..Line 9)
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
    let mut view = div(children.collect::<Vec<_>>());
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

/// Verifies horizontal overflow scrolls and renders a bottom gutter.
///
/// # Example Under Test
///
/// ```text
/// 5x2 div(8x1 text("ABCDEFGH"))
/// overflow: scroll clip
/// ScrollRight
/// ```
///
/// # Assertions
///
/// - Horizontal overflow retains a three-column scroll range.
/// - The first wheel step advances only the horizontal offset.
/// - Redrawing reveals the next source column.
/// - The bottom row contains the horizontal scrollbar.
#[test]
fn horizontal_overflow_scrolls_with_horizontal_wheel_events() -> Result<()> {
    let child = text("ABCDEFGH").with_inline_style(TuiStyle::new().size(LayoutSize::new(
        Dimension::from(Length::cells(8.0)),
        Dimension::from(Length::cells(1.0)),
    )));
    let mut view = div([child]).with_inline_style(
        TuiStyle::new()
            .size(LayoutSize::new(
                Dimension::from(Length::cells(5.0)),
                Dimension::from(Length::cells(2.0)),
            ))
            .overflow(Axes::new(Overflow::Scroll, Overflow::Clip)),
    );
    let mut terminal = Terminal::new(TestBackend::new(5, 2))?;

    draw_view(&mut terminal, &view)?;
    let metadata = view.style_metadata().expect("expected div metadata");
    assert_eq!(metadata.max_scroll_offsets(), Axes::new(3, 0));
    assert_eq!(cell_symbol(&terminal, 0, 0, 5), "A");
    assert_ne!(cell_symbol(&terminal, 0, 1, 5), " ");

    view.handle_event(mouse(MouseEventKind::ScrollRight, 0, 0))?;
    assert_eq!(
        view.style_metadata()
            .expect("expected div metadata")
            .scroll_offsets(),
        Axes::new(1, 0)
    );
    draw_view(&mut terminal, &view)?;
    assert_eq!(cell_symbol(&terminal, 0, 0, 5), "B");
    Ok(())
}

/// Verifies overflow modes establish independent per-axis scroll ranges.
///
/// # Example Under Test
///
/// ```text
/// 5x3 div(8x4 child)
/// overflow combinations:
///   hidden clip
///   clip hidden
///   scroll scroll
/// ```
///
/// # Assertions
///
/// - Horizontal hidden overflow scrolls without enabling vertical scrolling.
/// - Vertical hidden overflow scrolls without enabling horizontal scrolling.
/// - Two scroll gutters reduce the viewport to `4x2`.
/// - The bottom-right gutter corner remains unpainted by either scrollbar.
#[test]
fn overflow_modes_resolve_independently_on_both_axes() -> Result<()> {
    let fixed_child = || {
        text("content").with_inline_style(TuiStyle::new().size(LayoutSize::new(
            Dimension::from(Length::cells(8.0)),
            Dimension::from(Length::cells(4.0)),
        )))
    };
    let fixed_parent = |overflow| {
        div([fixed_child()]).with_inline_style(
            TuiStyle::new()
                .size(LayoutSize::new(
                    Dimension::from(Length::cells(5.0)),
                    Dimension::from(Length::cells(3.0)),
                ))
                .overflow(overflow),
        )
    };

    let horizontal = fixed_parent(Axes::new(Overflow::Hidden, Overflow::Clip));
    let vertical = fixed_parent(Axes::new(Overflow::Clip, Overflow::Hidden));
    let both = fixed_parent(Axes::all(Overflow::Scroll));
    let mut horizontal_terminal = Terminal::new(TestBackend::new(5, 3))?;
    let mut vertical_terminal = Terminal::new(TestBackend::new(5, 3))?;
    let mut both_terminal = Terminal::new(TestBackend::new(5, 3))?;

    draw_view(&mut horizontal_terminal, &horizontal)?;
    draw_view(&mut vertical_terminal, &vertical)?;
    draw_view(&mut both_terminal, &both)?;

    assert_eq!(
        horizontal
            .style_metadata()
            .expect("expected horizontal metadata")
            .max_scroll_offsets(),
        Axes::new(3, 0)
    );
    assert_eq!(
        vertical
            .style_metadata()
            .expect("expected vertical metadata")
            .max_scroll_offsets(),
        Axes::new(0, 1)
    );
    assert_eq!(
        both.style_metadata()
            .expect("expected two-axis metadata")
            .max_scroll_offsets(),
        Axes::new(4, 2)
    );
    assert_eq!(cell_symbol(&both_terminal, 4, 2, 5), " ");
    Ok(())
}

/// Verifies reconciliation retains two-axis overflow state and content extent.
///
/// # Example Under Test
///
/// ```text
/// render 5x3 scroll container with 8x4 child
/// ScrollRight, ScrollDown
/// reconcile compatible div
/// ```
///
/// # Assertions
///
/// - Both scroll offsets survive reconciliation.
/// - Both maximum offsets survive reconciliation.
/// - The measured content width and height survive reconciliation.
/// - A later smaller content extent clamps both retained offsets.
#[test]
fn reconciliation_retains_two_axis_overflow_metadata() -> Result<()> {
    let create_view = || {
        div([text("content").with_inline_style(TuiStyle::new().size(LayoutSize::new(
            Dimension::from(Length::cells(8.0)),
            Dimension::from(Length::cells(4.0)),
        )))])
        .with_inline_style(
            TuiStyle::new()
                .size(LayoutSize::new(
                    Dimension::from(Length::cells(5.0)),
                    Dimension::from(Length::cells(3.0)),
                ))
                .overflow(Axes::all(Overflow::Scroll)),
        )
    };
    let mut previous = create_view();
    let mut terminal = Terminal::new(TestBackend::new(5, 3))?;
    draw_view(&mut terminal, &previous)?;
    previous.handle_event(mouse(MouseEventKind::ScrollRight, 0, 0))?;
    previous.handle_event(mouse(MouseEventKind::ScrollDown, 0, 0))?;

    let previous_metadata = previous
        .style_metadata()
        .expect("expected previous metadata");
    let expected_offsets = previous_metadata.scroll_offsets();
    let expected_maximum = previous_metadata.max_scroll_offsets();
    let expected_extent = previous_metadata.content_extent();
    let mut next = create_view();
    leptatui::__private::__reconcile_view(&mut next, &previous);

    let next_metadata = next.style_metadata().expect("expected reconciled metadata");
    assert_eq!(next_metadata.scroll_offsets(), expected_offsets);
    assert_eq!(next_metadata.max_scroll_offsets(), expected_maximum);
    assert_eq!(next_metadata.content_extent(), expected_extent);

    let mut smaller = div([text("fit").with_inline_style(TuiStyle::new().size(
        LayoutSize::new(
            Dimension::from(Length::cells(4.0)),
            Dimension::from(Length::cells(2.0)),
        ),
    ))])
    .with_inline_style(
        TuiStyle::new()
            .size(LayoutSize::new(
                Dimension::from(Length::cells(5.0)),
                Dimension::from(Length::cells(3.0)),
            ))
            .overflow(Axes::all(Overflow::Scroll)),
    );
    leptatui::__private::__reconcile_view(&mut smaller, &previous);
    draw_view(&mut terminal, &smaller)?;
    assert_eq!(
        smaller
            .style_metadata()
            .expect("expected smaller metadata")
            .scroll_offsets(),
        Axes::all(0)
    );
    Ok(())
}

/// Verifies focus visibility scrolls horizontally to an offscreen button.
///
/// # Example Under Test
///
/// ```text
/// 8x3 flex div([6x2 button("One"), 6x2 button("Two")])
/// overflow: auto clip
/// Tab, Tab, render
/// ```
///
/// # Assertions
///
/// - The second button receives focus.
/// - Rendering advances the horizontal scroll offset.
/// - The focused button label is visible.
#[test]
fn focus_visibility_scrolls_horizontally() -> Result<()> {
    let button_style = TuiStyle::new()
        .size(LayoutSize::new(
            Dimension::from(Length::cells(6.0)),
            Dimension::from(Length::cells(2.0)),
        ))
        .flex_shrink(0.0);
    let mut view = div([
        button("One").with_inline_style(button_style),
        button("Two").with_inline_style(button_style),
    ])
    .with_inline_style(
        TuiStyle::new()
            .display(Display::Flex)
            .size(LayoutSize::new(
                Dimension::from(Length::cells(8.0)),
                Dimension::from(Length::cells(3.0)),
            ))
            .overflow(Axes::new(Overflow::Auto, Overflow::Clip)),
    );
    let mut terminal = Terminal::new(TestBackend::new(8, 3))?;

    view.handle_event(key(KeyCode::Tab))?;
    view.handle_event(key(KeyCode::Tab))?;
    draw_view(&mut terminal, &view)?;

    assert_eq!(button_focuses(&view), vec![false, true]);
    assert!(
        view.style_metadata()
            .expect("expected div metadata")
            .scroll_offsets()
            .x
            > 0
    );
    assert!(rendered_text(&terminal).contains("Two"));
    Ok(())
}
