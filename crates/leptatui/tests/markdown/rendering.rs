/// Verifies fixture documents render stable terminal fragments without snapshots.
///
/// # Example Under Test
///
/// ```text
/// core.md at 40x80
/// fallbacks.md at 48x80
/// ```
///
/// # Assertions
///
/// - Markdown H1 through H6 use the same repeated `#` heading hierarchy.
/// - Long Unicode prose wraps across multiple terminal rows.
/// - Ordered and unordered list markers remain visible.
/// - Link labels remain visible without appended destinations.
/// - Quote prefixes, rules, image fallbacks, literal HTML, and tables render visibly.
#[test]
fn markdown_fixtures_render_targeted_terminal_fragments() -> Result<()> {
    let core_document = markdown(CORE_FIXTURE);
    let core = render_view(core_document.as_view(), 40, 80)?;
    let core_lines = rendered_lines(&core);
    for expected_heading in [
        "# One",
        "## Two",
        "### Three",
        "#### Four",
        "##### Five",
        "###### Six",
    ] {
        assert!(
            core_lines
                .iter()
                .any(|line| line.starts_with(expected_heading))
        );
    }
    assert!(core_lines.iter().any(|line| line.contains("Unicode 界")));
    assert!(core_lines.iter().any(|line| line.contains("the guide")));
    assert!(
        !core_lines
            .iter()
            .any(|line| line.contains("https://example.com/guide"))
    );
    assert!(
        core_lines
            .iter()
            .any(|line| line.trim_start().starts_with("3."))
    );
    assert!(
        core_lines
            .iter()
            .any(|line| line.trim_start().starts_with("- Nested"))
    );
    assert!(
        core_lines
            .iter()
            .any(|line| line.trim_start().starts_with("7."))
    );

    let fallback_document = markdown(FALLBACKS_FIXTURE);
    let fallbacks = render_view(fallback_document.as_view(), 48, 80)?;
    let fallback_lines = rendered_lines(&fallbacks);
    assert!(
        fallback_lines
            .iter()
            .any(|line| line.starts_with("│ Alpha"))
    );
    assert!(
        fallback_lines
            .iter()
            .any(|line| line.starts_with("│ │ Inner"))
    );
    assert!(fallback_lines.iter().any(|line| line.starts_with("────")));
    assert!(
        fallback_lines
            .iter()
            .any(|line| line.contains("Image: diagram"))
    );
    assert!(fallback_lines.iter().any(|line| line.contains("<section>")));
    assert!(fallback_lines.iter().any(|line| line.contains("Default")));
    assert!(fallback_lines.iter().any(|line| line.contains("gamma")));

    Ok(())
}

/// Verifies Markdown syntax options affect semantic and rendered code output.
///
/// # Example Under Test
///
/// ```text
/// code.md with the light syntax theme and line numbers enabled
/// ```
///
/// # Assertions
///
/// - Every code block records the requested syntax theme and line-number behavior.
/// - Known `rust` and `rs` selectors contain highlighted spans.
/// - The unknown selector retains plain unstyled source.
/// - Rendered code shows language titles, line-number gutters, and wrapped Unicode source.
#[test]
fn markdown_code_fixture_applies_options_and_renders_highlighting() -> Result<()> {
    let options = MarkdownOptions::default()
        .syntax_theme(SyntaxTheme::Light)
        .line_numbers(true);
    let view = markdown_with_options(CODE_FIXTURE, options);
    let document = view
        .downcast_ref::<DivView>()
        .expect("Markdown document should have a Div root");
    let code_blocks = document
        .children()
        .iter()
        .filter_map(AnyView::downcast_ref::<CodeBlockView>)
        .collect::<Vec<_>>();
    assert_eq!(code_blocks.len(), 6);

    for child in &code_blocks {
        assert!(child.has_line_numbers());
        assert_eq!(child.selected_syntax_theme(), SyntaxTheme::Light);
    }

    for index in [0, 1] {
        assert!(
            code_blocks[index]
                .highlighted_lines()
                .iter()
                .flat_map(|line| &line.spans)
                .any(|span| span.style.fg.is_some()),
        );
    }

    assert!(
        code_blocks[2]
            .highlighted_lines()
            .iter()
            .flat_map(|line| &line.spans)
            .all(|span| span.style.fg.is_none()),
    );

    let rendered = render_view(view.as_view(), 24, 80)?;
    let lines = rendered_lines(&rendered);
    assert!(lines.iter().any(|line| line.contains("rust")));
    assert!(lines.iter().any(|line| line.contains("rs")));
    assert!(lines.iter().any(|line| line.contains("unknown-language")));
    assert!(lines.iter().any(|line| line.contains("1 │")));
    assert!(lines.iter().any(|line| line.contains('界')));

    Ok(())
}

/// Verifies fenced Markdown code fills the complete code-block interior.
///
/// # Example Under Test
///
/// ````text
/// ```rust
/// let value = true;
/// ```
/// ````
///
/// # Assertions
///
/// - The syntax background extends from source text through the trailing row width.
/// - The trailing logical blank line retains the same syntax background.
/// - The surrounding border does not receive the syntax background.
#[test]
fn markdown_code_background_fills_the_block_interior() -> Result<()> {
    let source = "```rust\nlet value = true;\n```\n";
    let document = markdown(source);
    let terminal = render_view(document.as_view(), 24, 4)?;
    let cells = terminal.backend().buffer().content();
    let background_at = |x: usize, y: usize| cells[y * 24 + x].bg;
    let code_background = background_at(1, 1);

    assert_eq!(background_at(22, 1), code_background);
    assert_eq!(background_at(1, 2), code_background);
    assert_ne!(background_at(0, 0), code_background);

    Ok(())
}

/// Verifies empty input and zero-sized Markdown viewports render safely.
///
/// # Example Under Test
///
/// ```text
/// empty.md
/// code.md at widths 0 through 2
/// ```
///
/// # Assertions
///
/// - The empty fixture produces an empty semantic document column.
/// - Empty input renders successfully in a zero-sized terminal.
/// - Code-heavy Markdown measures and renders without panicking at widths zero through two.
#[test]
fn markdown_fixtures_handle_empty_and_zero_sized_viewports() -> Result<()> {
    let empty = markdown(EMPTY_FIXTURE);
    assert!(
        empty
            .downcast_ref::<DivView>()
            .is_some_and(|layout| layout.children().is_empty())
    );
    let empty_terminal = render_view(empty.as_view(), 0, 0)?;
    assert!(rendered_lines(&empty_terminal).is_empty());

    let code = markdown(CODE_FIXTURE);
    for width in 0..=2 {
        let mut terminal = Terminal::new(TestBackend::new(width, 2))?;
        let mut render_result = Ok(());
        terminal.draw(|frame| {
            let mut ctx = RenderCtx::new(frame);
            let _ = code.__min_height(&mut ctx);
            render_result = code.render(&mut ctx);
        })?;
        render_result?;
    }

    Ok(())
}
