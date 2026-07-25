/// Verifies image builders store source, fallback text, and selector metadata.
///
/// # Example Under Test
///
/// ```text
/// image("assets/logo.png")
///     .alt("Project logo")
///     .with_id("logo")
///     .with_classes("media primary")
/// ```
///
/// # Assertions
///
/// - The image source is path-backed.
/// - Fallback text is retained.
/// - The metadata view type is `Image`.
/// - Standard selector metadata is retained.
#[test]
fn image_builder_stores_source_alt_and_selector_metadata() {
    let style = TuiStyle::new().foreground(Color::Yellow);
    let view = image("assets/logo.png")
        .alt("Project logo")
        .with_id("logo")
        .with_classes("media primary")
        .with_inline_style(style);

    assert_eq!(view.source(), &ImageSource::Path("assets/logo.png".into()));
    assert_eq!(view.alt_text(), Some("Project logo"));
    assert_eq!(view.metadata().view_type(), ViewType::Image);
    assert_eq!(view.metadata().id(), Some("logo"));
    assert_eq!(
        view.metadata().classes(),
        &[String::from("media"), String::from("primary")]
    );
    assert_eq!(view.metadata().inline_style(), Some(style));
}

/// Verifies image fallback rendering prefers caller-provided alt text.
///
/// # Example Under Test
///
/// ```text
/// image("missing.png").alt("Project logo")
/// TestBackend
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The fallback text is rendered into the test backend.
/// - No escape protocol bytes are written into the text buffer.
///
/// # Why
///
/// Test backends must remain deterministic even when terminal image protocols
/// are unavailable.
#[test]
fn image_fallback_renders_alt_text_on_test_backend() -> Result<()> {
    let backend = TestBackend::new(24, 2);
    let mut terminal = Terminal::new(backend)?;
    let view = image("missing.png").alt("Project logo");

    draw_view(&mut terminal, &view)?;

    let rendered = rendered_text(&terminal);
    assert!(rendered.contains("Project logo"));
    assert!(!rendered.contains('\u{1b}'));

    Ok(())
}

/// Verifies image fallback rendering has deterministic text without alt text.
///
/// # Example Under Test
///
/// ```text
/// image("missing.png")
/// TestBackend
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The rendered fallback text matches the runtime deterministic support
///   message.
#[test]
fn image_fallback_without_alt_uses_support_message() -> Result<()> {
    let backend = TestBackend::new(40, 2);
    let mut terminal = Terminal::new(backend)?;
    let view = image("missing.png");

    draw_view(&mut terminal, &view)?;

    let expected = "terminal image support is unavailable";
    assert!(rendered_text(&terminal).contains(expected));

    Ok(())
}

/// Verifies image type styles apply to fallback text.
///
/// # Example Under Test
///
/// ```text
/// Image { fg: Green }
/// image("missing.png").alt("Logo")
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The fallback text resolves styles through `ViewType::Image`.
#[test]
fn image_type_styles_apply_to_fallback_text() -> Result<()> {
    let backend = TestBackend::new(8, 1);
    let mut terminal = Terminal::new(backend)?;
    let view = image("missing.png").alt("Logo");
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::view_type(ViewType::Image),
        TuiStyle::new().foreground(Color::Green),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    let (fg, _) = cell_colors(&terminal, 0, 0, 8);
    assert_eq!(fg, Color::Green);

    Ok(())
}

/// Verifies image fallback text inherits parent text styles.
///
/// # Example Under Test
///
/// ```text
/// Form { fg: Green }
/// form([image("missing.png").alt("Logo")])
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The image fallback cell inherits foreground color from the form.
#[test]
fn image_fallback_text_inherits_parent_text_style() -> Result<()> {
    let backend = TestBackend::new(8, 1);
    let mut terminal = Terminal::new(backend)?;
    let view = form([image("missing.png").alt("Logo")]);
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::view_type(ViewType::Form),
        TuiStyle::new().foreground(Color::Green),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    let (fg, _) = cell_colors(&terminal, 0, 0, 8);
    assert_eq!(fg, Color::Green);

    Ok(())
}

/// Verifies stylesheet image size controls image minimum height.
///
/// # Example Under Test
///
/// ```text
/// .thumbnail { image_size: TuiSize::new(6, 3) }
/// image("missing.png").with_classes("thumbnail")
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The image minimum height is the stylesheet-declared image height.
#[test]
fn image_stylesheet_size_controls_min_height() -> Result<()> {
    let backend = TestBackend::new(12, 4);
    let mut terminal = Terminal::new(backend)?;
    let view = image("missing.png").with_classes("thumbnail");
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::class("thumbnail"),
        TuiStyle::new().image_size(TuiSize::new(6, 3)),
    );
    let mut min_height = 0;

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        min_height = ctx.__with_stylesheet(&stylesheet, |ctx| view.__min_height(ctx));
    })?;

    assert_eq!(min_height, 3);

    Ok(())
}

/// Verifies stylesheet image size constrains fallback rendering.
///
/// # Example Under Test
///
/// ```text
/// .thumbnail { image_size: TuiSize::new(3, 1) }
/// image("missing.png").alt("ABCDE").with_classes("thumbnail")
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - Fallback text renders only inside the styled image area.
#[test]
fn image_stylesheet_size_constrains_fallback_area() -> Result<()> {
    let backend = TestBackend::new(8, 2);
    let mut terminal = Terminal::new(backend)?;
    let view = image("missing.png").alt("ABCDE").with_classes("thumbnail");
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::class("thumbnail"),
        TuiStyle::new().image_size(TuiSize::new(3, 1)),
    );
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert_eq!(cell_symbol(&terminal, 0, 0, 8), "A");
    assert_eq!(cell_symbol(&terminal, 1, 0, 8), "B");
    assert_eq!(cell_symbol(&terminal, 2, 0, 8), "C");
    assert_eq!(cell_symbol(&terminal, 3, 0, 8), " ");
    assert_eq!(cell_symbol(&terminal, 0, 1, 8), " ");

    Ok(())
}

/// Verifies horizontally clipped images use the container's retained viewport.
///
/// # Example Under Test
///
/// ```text
/// 4x1 hidden-overflow div
/// 6x1 image fallback: ABCDEF
/// ScrollRight
/// ```
///
/// # Assertions
///
/// - Initial painting copies the first four fallback cells.
/// - Horizontal scrolling advances the image source by one cell.
/// - The image remains clipped to the four-cell viewport.
#[test]
fn image_fallback_clipping_uses_retained_viewport_geometry() -> Result<()> {
    let child = image("missing.png")
        .alt("ABCDEF")
        .with_inline_style(
            TuiStyle::new()
                .image_size(TuiSize::new(6, 1))
                .size(LayoutSize::new(
                    Dimension::from(Length::cells(6.0)),
                    Dimension::from(Length::cells(1.0)),
                ))
                .flex_shrink(0.0),
        );
    let mut view = div([child]).with_inline_style(
        TuiStyle::new()
            .display(Display::Flex)
            .size(LayoutSize::new(
                Dimension::from(Length::cells(4.0)),
                Dimension::from(Length::cells(1.0)),
            ))
            .overflow(Axes::new(Overflow::Hidden, Overflow::Clip)),
    );
    let mut terminal = Terminal::new(TestBackend::new(4, 1))?;

    draw_view(&mut terminal, &view)?;
    assert_eq!(rendered_text(&terminal), "ABCD");

    view.handle_event(mouse(MouseEventKind::ScrollRight, 0, 0))?;
    draw_view(&mut terminal, &view)?;
    assert_eq!(rendered_text(&terminal), "BCDE");
    Ok(())
}
