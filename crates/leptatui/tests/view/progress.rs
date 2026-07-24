/// Verifies progress bar builders store value, label, and selector metadata.
///
/// # Example Under Test
///
/// ```text
/// progress_bar(0.5)
///     .label("Loading")
///     .with_id("upload")
///     .with_classes("meter primary")
///     .with_inline_style(yellow)
/// ```
///
/// # Assertions
///
/// - The progress value is retained.
/// - The optional label is retained.
/// - The metadata view type is `ProgressBar`.
/// - Standard selector metadata is retained.
/// - Out-of-range builder values are clamped.
#[test]
fn progress_bar_builder_stores_value_label_and_selector_metadata() {
    let style = TuiStyle::new().foreground(Color::Yellow);
    let view = progress_bar(0.5)
        .label("Loading")
        .with_id("upload")
        .with_classes("meter primary")
        .with_inline_style(style);

    assert_eq!(view.value(), 0.5);
    assert_eq!(view.label_text(), Some("Loading"));
    assert_eq!(view.metadata().view_type(), ViewType::ProgressBar);
    assert_eq!(view.metadata().id(), Some("upload"));
    assert_eq!(
        view.metadata().classes(),
        &[String::from("meter"), String::from("primary")]
    );
    assert_eq!(view.metadata().inline_style(), Some(style));

    assert_eq!(progress_bar(1.5).value(), 1.0);
    assert_eq!(progress_bar(f64::NAN).value(), 0.0);
}

/// Verifies empty, partial, and full progress values render as gauges.
///
/// # Example Under Test
///
/// ```text
/// progress_bar(0.0)
/// progress_bar(0.5)
/// progress_bar(1.0)
/// ```
///
/// # Assertions
///
/// - Empty progress renders without filled cells.
/// - Partial progress renders filled cells.
/// - Full progress fills both edges around Ratatui's centered label.
#[test]
fn progress_bar_renders_empty_partial_and_full_values() -> Result<()> {
    let mut empty_terminal = Terminal::new(TestBackend::new(10, 1))?;
    draw_view(&mut empty_terminal, &progress_bar(0.0))?;
    assert!(!rendered_text(&empty_terminal).contains(symbol_block::FULL));

    let mut partial_terminal = Terminal::new(TestBackend::new(10, 1))?;
    draw_view(&mut partial_terminal, &progress_bar(0.5))?;
    assert_eq!(cell_symbol(&partial_terminal, 0, 0, 10), symbol_block::FULL);
    assert_ne!(cell_symbol(&partial_terminal, 9, 0, 10), symbol_block::FULL);

    let mut full_terminal = Terminal::new(TestBackend::new(10, 1))?;
    draw_view(&mut full_terminal, &progress_bar(1.0))?;
    assert_eq!(cell_symbol(&full_terminal, 0, 0, 10), symbol_block::FULL);
    assert_eq!(cell_symbol(&full_terminal, 9, 0, 10), symbol_block::FULL);

    Ok(())
}

/// Verifies progress values are clamped before rendering.
///
/// # Example Under Test
///
/// ```text
/// progress_bar(-0.5)
/// progress_bar(1.5)
/// progress_bar(f64::NAN)
/// ```
///
/// # Assertions
///
/// - Underflow renders the same as `0.0`.
/// - Overflow renders the same as `1.0`.
/// - Non-finite progress renders the same as `0.0`.
#[test]
fn progress_bar_clamps_values_before_rendering() -> Result<()> {
    let mut underflow = Terminal::new(TestBackend::new(10, 1))?;
    let mut empty = Terminal::new(TestBackend::new(10, 1))?;
    draw_view(&mut underflow, &progress_bar(-0.5))?;
    draw_view(&mut empty, &progress_bar(0.0))?;
    assert_eq!(rendered_text(&underflow), rendered_text(&empty));

    let mut overflow = Terminal::new(TestBackend::new(10, 1))?;
    let mut full = Terminal::new(TestBackend::new(10, 1))?;
    draw_view(&mut overflow, &progress_bar(1.5))?;
    draw_view(&mut full, &progress_bar(1.0))?;
    assert_eq!(rendered_text(&overflow), rendered_text(&full));

    let mut non_finite = Terminal::new(TestBackend::new(10, 1))?;
    let mut empty_again = Terminal::new(TestBackend::new(10, 1))?;
    draw_view(&mut non_finite, &progress_bar(f64::NAN))?;
    draw_view(&mut empty_again, &progress_bar(0.0))?;
    assert_eq!(rendered_text(&non_finite), rendered_text(&empty_again));

    Ok(())
}

/// Verifies progress bar labels render over the gauge.
///
/// # Example Under Test
///
/// ```text
/// progress_bar(0.5).label("Uploading")
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The caller-provided label appears in the rendered buffer.
#[test]
fn progress_bar_renders_optional_label() -> Result<()> {
    let backend = TestBackend::new(20, 1);
    let mut terminal = Terminal::new(backend)?;
    let view = progress_bar(0.5).label("Uploading");

    draw_view(&mut terminal, &view)?;

    assert!(rendered_text(&terminal).contains("Uploading"));

    Ok(())
}

/// Verifies progress bar type styles apply to the gauge.
///
/// # Example Under Test
///
/// ```text
/// ProgressBar { fg: Green, bg: Blue }
/// progress_bar(1.0).label("Done")
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The gauge resolves styles through `ViewType::ProgressBar`.
#[test]
fn progress_bar_type_styles_apply_to_gauge() -> Result<()> {
    let backend = TestBackend::new(12, 1);
    let mut terminal = Terminal::new(backend)?;
    let view = progress_bar(1.0).label("Done");
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::view_type(ViewType::ProgressBar),
        TuiStyle::new()
            .foreground(Color::Green)
            .background(Color::Blue),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    let (fg, bg) = cell_colors(&terminal, 0, 0, 12);
    assert_eq!(fg, Color::Green);
    assert_eq!(bg, Color::Blue);

    Ok(())
}

/// Verifies progress bars do not participate in built-in focus traversal.
///
/// # Example Under Test
///
/// ```text
/// div([progress_bar(0.5), button("Save")])
/// Tab
/// ```
///
/// # Assertions
///
/// - Only the button is counted as focusable.
/// - Tab focuses the button and skips the progress bar.
#[test]
fn progress_bar_is_not_focusable() -> Result<()> {
    let mut view = div((progress_bar(0.5), button("Save")));

    assert_eq!(view.__focusable_count(), 1);
    assert_eq!(control_focuses(&view), vec![false]);
    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Tab))?,
        KeyControl::Handled
    );
    assert_eq!(control_focuses(&view), vec![true]);

    Ok(())
}
