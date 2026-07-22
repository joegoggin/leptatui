/// Verifies Markdown documents use the existing vertical scroll commands.
///
/// # Example Under Test
///
/// ```text
/// ten Markdown paragraphs rendered into a 3-row terminal
/// Down, Up, PageDown, PageUp, G, gg
/// ```
///
/// # Assertions
///
/// - Rendering establishes an overflowing scroll range on the document column.
/// - Arrow keys move one row down and up.
/// - Page keys move five rows down and up.
/// - `G` reaches the maximum offset and `gg` returns to zero.
#[test]
fn markdown_documents_use_existing_vertical_scroll_keys() -> Result<()> {
    let source = (1..=10)
        .map(|index| format!("Paragraph {index}."))
        .collect::<Vec<_>>()
        .join("\n\n");
    let mut view = markdown(source);
    let mut terminal = Terminal::new(TestBackend::new(20, 3))?;
    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    let (offset, max_offset) = markdown_scroll_state(&view);
    assert_eq!(offset, 0);
    assert!(max_offset >= 6);

    view.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))?;
    assert_eq!(markdown_scroll_state(&view).0, 1);
    view.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))?;
    assert_eq!(markdown_scroll_state(&view).0, 0);

    view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?;
    assert_eq!(markdown_scroll_state(&view).0, 5);
    view.handle_key_event(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE))?;
    assert_eq!(markdown_scroll_state(&view).0, 0);

    view.handle_key_event(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE))?;
    assert_eq!(markdown_scroll_state(&view).0, max_offset);
    view.handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))?;
    view.handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))?;
    assert_eq!(markdown_scroll_state(&view).0, 0);

    Ok(())
}

/// Verifies Markdown headings map to every semantic heading level.
///
/// # Example Under Test
///
/// ```text
/// # One
/// ## Two
/// ### Three
/// #### Four
/// ##### Five
/// ###### Six
/// ```
///
/// # Assertions
///
/// - Parsing succeeds without a fallible API.
/// - H1 through H6 appear in source order.
/// - Every heading retains its text content.
/// - Empty separator paragraphs retain one terminal row between headings.
#[test]
fn markdown_maps_all_heading_levels() {
    let source = concat!(
        "# One\n\n",
        "## Two\n\n",
        "### Three\n\n",
        "#### Four\n\n",
        "##### Five\n\n",
        "###### Six\n",
    );

    assert_eq!(
        markdown(source),
        column(separate_blocks(views![
            h1("One"),
            h2("Two"),
            h3("Three"),
            h4("Four"),
            h5("Five"),
            h6("Six"),
        ])),
    );
}

/// Verifies Markdown paragraphs retain both line-break forms and Unicode.
///
/// # Example Under Test
///
/// ```text
/// Soft
/// break\
/// hard 界 `code`
/// ```
///
/// # Assertions
///
/// - Parsing succeeds without a fallible API.
/// - Soft and hard breaks become explicit text line boundaries.
/// - Unicode remains intact and inline code uses reverse-video styling.
#[test]
fn markdown_preserves_paragraph_breaks_and_unicode() {
    let source = "Soft\nbreak  \nhard 界 `code`\n";

    assert_eq!(
        markdown(source),
        column([paragraph(Text::from(vec![
            Line::raw("Soft"),
            Line::raw("break"),
            Line::from(vec![
                Span::raw("hard 界 "),
                Span::styled("code", Style::new().add_modifier(Modifier::REVERSED),),
            ]),
        ]))]),
    );
}

/// Verifies Markdown blocks retain a blank terminal row between them.
///
/// # Example Under Test
///
/// ```text
/// One
///
/// Two
/// ```
///
/// # Assertions
///
/// - Parsing inserts one empty semantic paragraph between the blocks.
/// - Rendering retains the empty row between the visible paragraph rows.
#[test]
fn markdown_separates_blocks_with_blank_terminal_rows() -> Result<()> {
    let source = "One\n\nTwo\n";

    assert_eq!(
        markdown(source),
        column([paragraph("One"), paragraph(""), paragraph("Two")]),
    );
    assert_eq!(
        rendered_markdown_lines(source, 8, 3)?,
        ["One     ", "        ", "Two     "],
    );

    Ok(())
}
