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
fn semantic_rendering_preserves_rich_text_span_styles() -> leptatui::app::Result<()> {
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
fn heading_rendering_preserves_rich_text_styles_and_hanging_indent() -> leptatui::app::Result<()> {
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
fn semantic_text_variants_wrap_and_report_intrinsic_size() -> leptatui::app::Result<()> {
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
        let mut min_height = 0.0;
        terminal.draw(|frame| {
            let mut ctx = RenderCtx::new(frame);
            min_height = measure_view_in_area(&view, &mut ctx).height;
        })?;
        draw_view(&mut terminal, &view)?;

        assert_eq!(min_height, 2.0);
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
fn semantic_headings_handle_zero_and_narrow_widths() -> leptatui::app::Result<()> {
    for width in 0..=8 {
        let view = h6("Heading");
        let mut terminal = Terminal::new(TestBackend::new(width, 2))?;
        let mut min_height = 0.0;
        let mut render_result = Ok(());
        terminal.draw(|frame| {
            let mut ctx = RenderCtx::new(frame);
            min_height = measure_view_in_area(&view, &mut ctx).height;
            render_result = view.render(&mut ctx);
        })?;
        render_result?;

        if width == 0 {
            assert_eq!(min_height, 0.0);
        } else {
            assert!(min_height >= 1.0);
            assert_eq!(cell_symbol(&terminal, 0, 0, width), "#");
        }
        if (1..=7).contains(&width) {
            assert!(min_height > 1.0);
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
/// div([paragraph("界界界"), text("End")])
/// terminal width = 4
/// ```
///
/// # Assertions
///
/// - Two double-width characters fit on the first row.
/// - The third character wraps to the second row.
/// - The following text view renders after both paragraph rows.
#[test]
fn semantic_text_wraps_unicode_and_reserves_parent_layout_height() -> leptatui::app::Result<()> {
    let view = div((paragraph("界界界"), text("End")));
    let mut terminal = Terminal::new(TestBackend::new(4, 3))?;

    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 0, 0, 4), "界");
    assert_eq!(cell_symbol(&terminal, 2, 0, 4), "界");
    assert_eq!(cell_symbol(&terminal, 0, 1, 4), "界");
    assert_eq!(symbol_position(&terminal, "E", 4), (0, 2));

    Ok(())
}

/// Verifies semantic text clips overflow and tolerates zero-width flex items.
///
/// # Example Under Test
///
/// ```text
/// paragraph("One Two") in a 4x1 terminal
/// div([paragraph("A"), paragraph("B")]) in a 1x1 terminal
/// ```
///
/// # Assertions
///
/// - Content beyond the one-row render area is clipped.
/// - Rendering a flex row that assigns zero width to one child succeeds.
/// - The narrow row reports a one-row intrinsic size.
#[test]
fn semantic_text_clips_overflow_and_handles_zero_width_flex_items() -> leptatui::app::Result<()> {
    let mut clipped = Terminal::new(TestBackend::new(4, 1))?;
    draw_view(&mut clipped, &paragraph("One Two"))?;
    assert_eq!(symbol_position(&clipped, "O", 4), (0, 0));
    assert!(symbol_position_opt(&clipped, "T", 4).is_none());

    let narrow_view = div([paragraph("A"), paragraph("B")])
        .with_inline_style(TuiStyle::new().display(Display::Flex));
    let mut narrow = Terminal::new(TestBackend::new(1, 1))?;
    let mut min_height = 0.0;
    narrow.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        min_height = measure_view_in_area(&narrow_view, &mut ctx).height;
    })?;
    draw_view(&mut narrow, &narrow_view)?;
    assert_eq!(min_height, 1.0);

    Ok(())
}
