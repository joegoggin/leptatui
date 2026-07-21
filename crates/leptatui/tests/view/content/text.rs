/// Verifies a block view renders its child text.
///
/// # Example Under Test
///
/// ```text
/// block(text("Hello"))
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The view render call succeeds.
/// - The rendered buffer contains `Hello`.
#[test]
fn renders_block_and_text_views() -> Result<()> {
    let backend = TestBackend::new(24, 5);
    let mut terminal = Terminal::new(backend)?;
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = block(text("Hello")).render(&mut ctx);
    })?;
    render_result?;

    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();

    assert!(rendered.contains("Hello"));

    Ok(())
}

/// Verifies text views render with stylesheet-resolved colors.
///
/// # Example Under Test
///
/// ```text
/// text("Hi").with_classes("accent")
/// Stylesheet::new().rule(StyleSelector::class("accent"), yellow on blue)
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The view render call succeeds.
/// - The rendered `H` cell exists.
/// - The rendered `H` cell has a yellow foreground.
/// - The rendered `H` cell has a blue background.
#[test]
fn renders_text_with_resolved_stylesheet_style() -> Result<()> {
    let backend = TestBackend::new(12, 3);
    let mut terminal = Terminal::new(backend)?;
    let view = text("Hi").with_classes("accent");
    let stylesheet = Stylesheet::new().rule(
        StyleSelector::class("accent"),
        TuiStyle::new()
            .foreground(Color::Yellow)
            .background(Color::Blue),
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
    assert_eq!(cell.bg, Color::Blue);

    Ok(())
}

/// Verifies text wraps to the available render width.
///
/// # Example Under Test
///
/// ```text
/// text("Hello World")
/// terminal width = 6
/// ```
///
/// # Assertions
///
/// - The terminal draw call succeeds.
/// - The view render call succeeds.
/// - `Hello` starts on the first row.
/// - `World` starts on the second row.
#[test]
fn text_wraps_to_available_render_width() -> Result<()> {
    let backend = TestBackend::new(6, 3);
    let mut terminal = Terminal::new(backend)?;
    let view = text("Hello World");
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(symbol_position(&terminal, "H", 6), (0, 0));
    assert_eq!(symbol_position(&terminal, "W", 6), (0, 1));

    Ok(())
}

/// Verifies semantic text builders retain rich text and selector metadata.
///
/// # Example Under Test
///
/// ```text
/// h1(Text::from(Line::from([Span::raw("Guide"), Span::styled("!", yellow)])))
/// h2("Guide") through h6("Guide")
/// paragraph("Guide")
/// ```
///
/// # Assertions
///
/// - Every builder creates its corresponding semantic view variant.
/// - Every view retains the supplied rich text.
/// - Every view stores its corresponding selector view type.
#[test]
fn semantic_text_builders_store_rich_text_and_metadata() {
    let content = Text::from(Line::from(vec![
        Span::raw("Guide"),
        Span::styled("!", Style::new().fg(Color::Yellow)),
    ]));
    let headings = [
        (h1(content.clone()), ViewType::H1),
        (h2(content.clone()), ViewType::H2),
        (h3(content.clone()), ViewType::H3),
        (h4(content.clone()), ViewType::H4),
        (h5(content.clone()), ViewType::H5),
        (h6(content.clone()), ViewType::H6),
    ];

    for (view, expected_type) in headings {
        assert_eq!(view.level().view_type(), expected_type);
        assert_eq!(view.content(), &content);
        assert_eq!(view.metadata().view_type(), expected_type);
    }

    let paragraph = paragraph(content.clone());
    assert_eq!(paragraph.content(), &content);
    assert_eq!(paragraph.metadata().view_type(), ViewType::Paragraph);
}

/// Verifies semantic text views render their documented hierarchy and modifiers.
///
/// # Example Under Test
///
/// ```text
/// h1("H1") through h6("H6")
/// paragraph("Paragraph")
/// ```
///
/// # Assertions
///
/// - H1 is bold and retains the terminal's default background.
/// - H2 is bold.
/// - H3 is bold and italic.
/// - H4 is italic.
/// - H5 is dim and italic.
/// - H6 is dim.
/// - H1 through H6 use Markdown-style `#` markers and no underline modifier.
/// - Paragraph has no default modifier.
#[test]
fn semantic_text_views_render_default_modifiers() -> Result<()> {
    let headings = [
        (h1("H1"), 1, Modifier::BOLD),
        (h2("H2"), 2, Modifier::BOLD),
        (h3("H3"), 3, Modifier::BOLD | Modifier::ITALIC),
        (h4("H4"), 4, Modifier::ITALIC),
        (h5("H5"), 5, Modifier::DIM | Modifier::ITALIC),
        (h6("H6"), 6, Modifier::DIM),
    ];

    for (view, level, expected_modifiers) in headings {
        let mut terminal = Terminal::new(TestBackend::new(16, 1))?;
        draw_view(&mut terminal, &view)?;

        let content_x = level + 1;
        for marker_x in 0..level {
            assert_eq!(cell_symbol(&terminal, marker_x, 0, 16), "#");
        }
        assert_eq!(cell_symbol(&terminal, content_x, 0, 16), "H");
        assert_eq!(
            cell_modifiers(&terminal, content_x, 0, 16),
            expected_modifiers
        );
        assert_eq!(cell_modifiers(&terminal, 0, 0, 16), expected_modifiers);
        assert!(!expected_modifiers.contains(Modifier::UNDERLINED));
        if level == 1 {
            assert_eq!(cell_colors(&terminal, content_x, 0, 16).1, Color::Reset);
        }
    }

    let mut paragraph_terminal = Terminal::new(TestBackend::new(16, 1))?;
    draw_view(&mut paragraph_terminal, &paragraph("Paragraph"))?;
    assert_eq!(
        cell_modifiers(&paragraph_terminal, 0, 0, 16),
        Modifier::empty()
    );

    Ok(())
}

/// Verifies semantic defaults remain below authored cascade declarations.
///
/// # Example Under Test
///
/// ```text
/// h1("Guide")
/// H1 { modifier: empty }
/// .title { modifier: italic }
/// inline modifier: dim
/// .title { modifier: crossed-out !important }
/// ```
///
/// # Assertions
///
/// - The H1 default resolves to bold without authored styles.
/// - A normal type rule can remove every default modifier.
/// - A class rule replaces the semantic default.
/// - An inline declaration replaces a normal class rule.
/// - An important rule replaces the inline declaration.
#[test]
fn semantic_defaults_have_low_cascade_precedence() {
    let theme = Default::default();
    let plain = h1("Guide");
    let default_style = Stylesheet::new().resolve(
        plain.style_metadata().expect("H1 metadata"),
        &[],
        TuiStyle::new(),
        &theme,
    );
    assert_eq!(default_style.modifiers, Some(Modifier::BOLD));

    let type_stylesheet = Stylesheet::new().rule(
        StyleSelector::view_type(ViewType::H1),
        TuiStyle::new().modifier(Modifier::empty()),
    );
    let type_style = type_stylesheet.resolve(
        plain.style_metadata().expect("H1 metadata"),
        &[],
        TuiStyle::new(),
        &theme,
    );
    assert_eq!(type_style.modifiers, Some(Modifier::empty()));

    let class_view = h1("Guide").with_classes("title");
    let class_stylesheet = Stylesheet::new().rule(
        StyleSelector::class("title"),
        TuiStyle::new().modifier(Modifier::ITALIC),
    );
    let class_style = class_stylesheet.resolve(
        class_view.style_metadata().expect("H1 metadata"),
        &[],
        TuiStyle::new(),
        &theme,
    );
    assert_eq!(class_style.modifiers, Some(Modifier::ITALIC));

    let inline_view = h1("Guide")
        .with_classes("title")
        .with_inline_style(TuiStyle::new().modifier(Modifier::DIM));
    let inline_style = class_stylesheet.resolve(
        inline_view.style_metadata().expect("H1 metadata"),
        &[],
        TuiStyle::new(),
        &theme,
    );
    assert_eq!(inline_style.modifiers, Some(Modifier::DIM));

    let important_stylesheet = Stylesheet::new().rule(
        StyleSelector::class("title"),
        StyleDeclarations::new().modifier_important(Modifier::CROSSED_OUT),
    );
    let important_style = important_stylesheet.resolve(
        inline_view.style_metadata().expect("H1 metadata"),
        &[],
        TuiStyle::new(),
        &theme,
    );
    assert_eq!(important_style.modifiers, Some(Modifier::CROSSED_OUT));
}
