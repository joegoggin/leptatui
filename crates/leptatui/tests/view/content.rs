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

/// Verifies semantic rendering preserves styles on rich-text spans.
///
/// # Example Under Test
///
/// ```text
/// paragraph([yellow reversed "Rich ", plain "body"])
/// terminal width = 5
/// ```
///
/// # Assertions
///
/// - The first span retains its yellow foreground and reversed modifier.
/// - The second span wraps to the second row.
/// - The second span does not inherit the first span's reversed modifier.
#[test]
fn semantic_rendering_preserves_rich_text_span_styles() -> Result<()> {
    let content = Text::from(Line::from(vec![
        Span::styled(
            "Rich ",
            Style::new()
                .fg(Color::Yellow)
                .add_modifier(Modifier::REVERSED),
        ),
        Span::raw("body"),
    ]));
    let view = paragraph(content);
    let mut terminal = Terminal::new(TestBackend::new(5, 2))?;

    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_colors(&terminal, 0, 0, 5).0, Color::Yellow);
    assert!(cell_modifiers(&terminal, 0, 0, 5).contains(Modifier::REVERSED));
    assert_eq!(symbol_position(&terminal, "b", 5), (0, 1));
    assert!(!cell_modifiers(&terminal, 0, 1, 5).contains(Modifier::REVERSED));

    Ok(())
}

/// Verifies heading decoration preserves rich-text span styles while wrapping.
///
/// # Example Under Test
///
/// ```text
/// h3([yellow reversed "Rich ", plain "body"])
/// terminal width = 9
/// ```
///
/// # Assertions
///
/// - The H3 marker contains three leading `#` cells.
/// - Both content rows begin after the marker gutter.
/// - The first span retains its yellow foreground and reversed modifier.
/// - The second span wraps without inheriting the first span's reversed modifier.
#[test]
fn heading_rendering_preserves_rich_text_styles_and_hanging_indent() -> Result<()> {
    let content = Text::from(Line::from(vec![
        Span::styled(
            "Rich ",
            Style::new()
                .fg(Color::Yellow)
                .add_modifier(Modifier::REVERSED),
        ),
        Span::raw("body"),
    ]));
    let mut terminal = Terminal::new(TestBackend::new(9, 2))?;

    draw_view(&mut terminal, &h3(content))?;

    assert_eq!(cell_symbol(&terminal, 0, 0, 9), "#");
    assert_eq!(cell_symbol(&terminal, 1, 0, 9), "#");
    assert_eq!(cell_symbol(&terminal, 2, 0, 9), "#");
    assert_eq!(cell_symbol(&terminal, 4, 0, 9), "R");
    assert_eq!(cell_colors(&terminal, 4, 0, 9).0, Color::Yellow);
    assert!(cell_modifiers(&terminal, 4, 0, 9).contains(Modifier::REVERSED));
    assert_eq!(cell_symbol(&terminal, 4, 1, 9), "b");
    assert!(!cell_modifiers(&terminal, 4, 1, 9).contains(Modifier::REVERSED));

    Ok(())
}

/// Verifies semantic headings wrap after their Markdown-style markers.
///
/// # Example Under Test
///
/// ```text
/// h1("One Two") through h6("One Two")
/// content width = 4
/// ```
///
/// # Assertions
///
/// - Every heading reports a two-row minimum height.
/// - Each marker contains one `#` per heading level with no leading indentation.
/// - Both heading rows begin after the complete marker gutter.
#[test]
fn semantic_text_variants_wrap_and_report_intrinsic_height() -> Result<()> {
    let headings = [
        (h1("One Two"), 1),
        (h2("One Two"), 2),
        (h3("One Two"), 3),
        (h4("One Two"), 4),
        (h5("One Two"), 5),
        (h6("One Two"), 6),
    ];

    for (view, level) in headings {
        let content_x = level + 1;
        let width = content_x + 4;
        let mut terminal = Terminal::new(TestBackend::new(width, 2))?;
        let mut min_height = 0;
        terminal.draw(|frame| {
            let mut ctx = RenderCtx::new(frame);
            min_height = view.__min_height(&mut ctx);
        })?;
        draw_view(&mut terminal, &view)?;

        assert_eq!(min_height, 2);
        for marker_x in 0..level {
            assert_eq!(cell_symbol(&terminal, marker_x, 0, width), "#");
        }
        assert_eq!(symbol_position(&terminal, "O", width), (content_x, 0));
        assert_eq!(symbol_position(&terminal, "T", width), (content_x, 1));
    }

    Ok(())
}

/// Verifies Markdown-style headings tolerate viewports narrower than their marker.
///
/// # Example Under Test
///
/// ```text
/// h6("Heading") at widths 0 through 8
/// ```
///
/// # Assertions
///
/// - Measurement and rendering succeed at every width.
/// - Any nonzero viewport renders the visible portion of the marker.
/// - Viewports narrower than the marker gutter render content on the next row.
/// - H6 renders all six hash cells when the viewport permits them.
/// - Heading content begins at cell seven when the viewport permits it.
#[test]
fn semantic_headings_handle_zero_and_narrow_widths() -> Result<()> {
    for width in 0..=8 {
        let view = h6("Heading");
        let mut terminal = Terminal::new(TestBackend::new(width, 2))?;
        let mut min_height = 0;
        let mut render_result = Ok(());
        terminal.draw(|frame| {
            let mut ctx = RenderCtx::new(frame);
            min_height = view.__min_height(&mut ctx);
            render_result = view.render(&mut ctx);
        })?;
        render_result?;

        if width == 0 {
            assert_eq!(min_height, 0);
        } else {
            assert!(min_height >= 1);
            assert_eq!(cell_symbol(&terminal, 0, 0, width), "#");
        }
        if (1..=7).contains(&width) {
            assert!(min_height > 1);
            assert_eq!(cell_symbol(&terminal, 0, 1, width), "H");
        }
        if width >= 6 {
            assert_eq!(cell_symbol(&terminal, 5, 0, width), "#");
        }
        if width >= 8 {
            assert_eq!(cell_symbol(&terminal, 7, 0, width), "H");
        }
    }

    Ok(())
}

/// Verifies semantic text uses Unicode display width during layout.
///
/// # Example Under Test
///
/// ```text
/// column([paragraph("界界界"), text("End")])
/// terminal width = 4
/// ```
///
/// # Assertions
///
/// - Two double-width characters fit on the first row.
/// - The third character wraps to the second row.
/// - The following text view renders after both paragraph rows.
#[test]
fn semantic_text_wraps_unicode_and_reserves_parent_layout_height() -> Result<()> {
    let view = column((paragraph("界界界"), text("End")));
    let mut terminal = Terminal::new(TestBackend::new(4, 3))?;

    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 0, 0, 4), "界");
    assert_eq!(cell_symbol(&terminal, 2, 0, 4), "界");
    assert_eq!(cell_symbol(&terminal, 0, 1, 4), "界");
    assert_eq!(symbol_position(&terminal, "E", 4), (0, 2));

    Ok(())
}

/// Verifies semantic text clips overflow and tolerates zero-width split areas.
///
/// # Example Under Test
///
/// ```text
/// paragraph("One Two") in a 4x1 terminal
/// row([paragraph("A"), paragraph("B")]) in a 1x1 terminal
/// ```
///
/// # Assertions
///
/// - Content beyond the one-row render area is clipped.
/// - Rendering a row that assigns zero width to one child succeeds.
/// - The narrow row reports a one-row minimum height.
#[test]
fn semantic_text_clips_overflow_and_handles_zero_width_splits() -> Result<()> {
    let mut clipped = Terminal::new(TestBackend::new(4, 1))?;
    draw_view(&mut clipped, &paragraph("One Two"))?;
    assert_eq!(symbol_position(&clipped, "O", 4), (0, 0));
    assert!(symbol_position_opt(&clipped, "T", 4).is_none());

    let narrow_view = row([paragraph("A"), paragraph("B")]);
    let mut narrow = Terminal::new(TestBackend::new(1, 1))?;
    let mut min_height = 0;
    narrow.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        min_height = narrow_view.__min_height(&mut ctx);
    })?;
    draw_view(&mut narrow, &narrow_view)?;
    assert_eq!(min_height, 1);

    Ok(())
}
