/// Verifies code-block builders retain source, highlighting, and display options.
///
/// # Example Under Test
///
/// ```text
/// code_block("fn main() {}")
///     .language("rs")
///     .line_numbers(true)
///     .syntax_theme(SyntaxTheme::Light)
/// ```
///
/// # Assertions
///
/// - Code blocks default to the dark theme with line numbers disabled.
/// - The configured language token and light theme are retained.
/// - The `rs` alias selects syntax highlighting instead of plain source spans.
/// - Code-block metadata uses [`ViewType::CodeBlock`].
#[test]
fn code_block_builder_retains_highlighted_lines_and_options() {
    let default = code_block("fn main() {}");
    assert!(!default.has_line_numbers());
    assert_eq!(default.selected_syntax_theme(), SyntaxTheme::Dark);

    let configured = code_block("fn main() {}")
        .language("rs")
        .line_numbers(true)
        .syntax_theme(SyntaxTheme::Light);
    assert_eq!(configured.source(), "fn main() {}");
    assert_eq!(configured.language_token(), Some("rs"));
    assert!(configured.has_line_numbers());
    assert_eq!(configured.selected_syntax_theme(), SyntaxTheme::Light);
    assert!(
        configured.highlighted_lines()[0]
            .spans
            .iter()
            .any(|span| span.style.fg.is_some())
    );
    assert_eq!(configured.metadata().view_type(), ViewType::CodeBlock);
}

/// Verifies aliases share highlighting and unknown languages fall back to plain source.
///
/// # Example Under Test
///
/// ```text
/// code_block("let value = 1;").language("rust")
/// code_block("let value = 1;").language("rs")
/// code_block("let value = 1;").language("not-a-language")
/// ```
///
/// # Assertions
///
/// - `rust` and `rs` produce the same retained syntax spans.
/// - An unknown token retains one unstyled span containing the complete source.
#[test]
fn code_block_recognizes_aliases_and_falls_back_for_unknown_languages() {
    let rust = code_block("let value = 1;").language("rust");
    let alias = code_block("let value = 1;").language("rs");
    let unknown = code_block("let value = 1;").language("not-a-language");

    let rust_lines = rust.highlighted_lines();
    let alias_lines = alias.highlighted_lines();
    let unknown_lines = unknown.highlighted_lines();

    assert_eq!(rust_lines, alias_lines);
    assert_eq!(unknown_lines.len(), 1);
    assert_eq!(unknown_lines[0].spans.len(), 1);
    assert_eq!(unknown_lines[0].spans[0].content, "let value = 1;");
    assert_eq!(unknown_lines[0].spans[0].style, Style::default());
}

/// Verifies code-block themes produce distinct syntax colors.
///
/// # Example Under Test
///
/// ```text
/// code_block("fn main() {}").language("rust")
/// code_block("fn main() {}").language("rust").syntax_theme(SyntaxTheme::Light)
/// ```
///
/// # Assertions
///
/// - Both themes retain highlighted spans.
/// - At least one corresponding syntax span has a different foreground or background color.
#[test]
fn code_block_dark_and_light_themes_produce_distinct_colors() {
    let dark = code_block("fn main() {}\nlet value = true;").language("rust");
    let light = dark.clone().syntax_theme(SyntaxTheme::Light);
    let dark_lines = dark.highlighted_lines();
    let light_lines = light.highlighted_lines();

    assert!(dark_lines.iter().any(|line| !line.spans.is_empty()));
    assert!(light_lines.iter().any(|line| !line.spans.is_empty()));
    assert!(dark_lines
        .iter()
        .flat_map(|line| &line.spans)
        .zip(light_lines.iter().flat_map(|line| &line.spans))
        .any(|(dark, light)| dark.style.fg != light.style.fg || dark.style.bg != light.style.bg));
}

/// Verifies language titles and logical-line gutters render inside the border.
///
/// # Example Under Test
///
/// ```text
/// code_block("one\ntwo").language("txt").line_numbers(true)
/// terminal size = 12x4
/// ```
///
/// # Assertions
///
/// - The language token appears in the top border title.
/// - One-based numbers and the rule separator render on both logical lines.
/// - Source content begins after the gutter.
#[test]
fn code_block_renders_language_title_and_line_number_gutter() -> Result<()> {
    let view = code_block("one\ntwo").language("txt").line_numbers(true);
    let mut terminal = Terminal::new(TestBackend::new(12, 4))?;

    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 1, 0, 12), "t");
    assert_eq!(cell_symbol(&terminal, 1, 1, 12), "1");
    assert_eq!(cell_symbol(&terminal, 3, 1, 12), "│");
    assert_eq!(cell_symbol(&terminal, 5, 1, 12), "o");
    assert_eq!(cell_symbol(&terminal, 1, 2, 12), "2");
    assert_eq!(cell_symbol(&terminal, 5, 2, 12), "t");

    Ok(())
}

/// Verifies code-block backgrounds fill the interior and honor authored overrides.
///
/// # Example Under Test
///
/// ```text
/// code_block("x").language("rust").padding(1)
/// code_block("x").language("unknown").background(Magenta).padding(1)
/// terminal size = 12x5
/// ```
///
/// # Assertions
///
/// - Dark and light syntax themes fill padding, code, trailing, and blank interior cells.
/// - Dark and light syntax-theme backgrounds remain distinct.
/// - An authored background overrides the selected theme for unknown-language fallback code.
/// - Border cells do not receive either syntax or authored interior backgrounds.
#[test]
fn code_block_background_fills_interior_and_honors_authored_override() -> Result<()> {
    let padding = TuiSpacing::uniform(1);
    let dark = code_block("x")
        .language("rust")
        .with_inline_style(TuiStyle::new().padding(padding));
    let mut dark_terminal = Terminal::new(TestBackend::new(12, 5))?;
    draw_view(&mut dark_terminal, &dark)?;
    let dark_background = cell_colors(&dark_terminal, 2, 2, 12).1;
    assert_ne!(dark_background, Color::Reset);
    for (x, y) in [(1, 1), (9, 2), (10, 2), (1, 3)] {
        assert_eq!(cell_colors(&dark_terminal, x, y, 12).1, dark_background);
    }
    assert_ne!(cell_colors(&dark_terminal, 0, 0, 12).1, dark_background);

    let light = dark.clone().syntax_theme(SyntaxTheme::Light);
    let mut light_terminal = Terminal::new(TestBackend::new(12, 5))?;
    draw_view(&mut light_terminal, &light)?;
    let light_background = cell_colors(&light_terminal, 2, 2, 12).1;
    assert_ne!(light_background, dark_background);
    assert_eq!(cell_colors(&light_terminal, 9, 2, 12).1, light_background);
    assert_eq!(cell_colors(&light_terminal, 1, 3, 12).1, light_background);
    assert_ne!(cell_colors(&light_terminal, 0, 0, 12).1, light_background);

    let authored = code_block("x")
        .language("unknown-language")
        .with_inline_style(TuiStyle::new().background(Color::Magenta).padding(padding));
    let mut authored_terminal = Terminal::new(TestBackend::new(12, 5))?;
    draw_view(&mut authored_terminal, &authored)?;
    for (x, y) in [(1, 1), (2, 2), (9, 2), (10, 2), (1, 3)] {
        assert_eq!(cell_colors(&authored_terminal, x, y, 12).1, Color::Magenta);
    }
    assert_ne!(cell_colors(&authored_terminal, 0, 0, 12).1, Color::Magenta);

    Ok(())
}

/// Verifies wrapped code preserves syntax styles and reserves document height.
///
/// # Example Under Test
///
/// ```text
/// div([code_block("let value = true;").language("rust"), text("End")])
/// terminal width = 10
/// ```
///
/// # Assertions
///
/// - The code block reports its wrapped content height plus both borders.
/// - Syntax-colored source continues onto later visual rows.
/// - The following document child begins after the code block's bottom border.
#[test]
fn code_block_wraps_highlighted_spans_and_reserves_intrinsic_size() -> Result<()> {
    let code = code_block("let value = true;").language("rust");
    let mut measured = Terminal::new(TestBackend::new(10, 8))?;
    let mut code_height = 0.0;
    measured.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        code_height = measure_view_in_area(&code, &mut ctx).height;
    })?;
    assert_eq!(code_height, 5.0);

    let document = div((code, text("End")));
    let mut terminal = Terminal::new(TestBackend::new(10, 6))?;
    draw_view(&mut terminal, &document)?;

    assert!(symbol_position(&terminal, "=", 10).1 >= 1);
    assert_eq!(symbol_position(&terminal, "E", 10), (0, 5));
    assert_eq!(cell_symbol(&terminal, 0, 4, 10), "└");

    Ok(())
}

/// Verifies empty, Unicode, narrow, and clipped code blocks render safely.
///
/// # Example Under Test
///
/// ```text
/// code_block("")
/// code_block("界界A") at width 5
/// code_block("abcdef") at widths 0 through 2 and height 2
/// ```
///
/// # Assertions
///
/// - Empty source contributes one content row between borders.
/// - Double-width Unicode wraps only at grapheme boundaries.
/// - Zero and extremely narrow widths do not panic during measurement or rendering.
/// - A two-row viewport clips the bottom border without failing.
#[test]
fn code_block_handles_empty_unicode_narrow_and_clipped_viewports() -> Result<()> {
    let empty = code_block("");
    let mut empty_terminal = Terminal::new(TestBackend::new(4, 3))?;
    let mut empty_height = 0.0;
    empty_terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        empty_height = measure_view_in_area(&empty, &mut ctx).height;
    })?;
    assert_eq!(empty_height, 3.0);

    let unicode = code_block("界界A");
    let mut unicode_terminal = Terminal::new(TestBackend::new(5, 4))?;
    draw_view(&mut unicode_terminal, &unicode)?;
    assert_eq!(cell_symbol(&unicode_terminal, 1, 1, 5), "界");
    assert_eq!(cell_symbol(&unicode_terminal, 1, 2, 5), "界");
    assert_eq!(cell_symbol(&unicode_terminal, 3, 2, 5), "A");

    for width in 0..=2 {
        let view = code_block("abcdef");
        let mut terminal = Terminal::new(TestBackend::new(width, 2))?;
        let mut render_result = Ok(());
        terminal.draw(|frame| {
            let mut ctx = RenderCtx::new(frame);
            let _ = measure_view_in_area(&view, &mut ctx);
            render_result = view.render(&mut ctx);
        })?;
        render_result?;
    }

    let clipped = code_block("abcdef");
    let mut clipped_terminal = Terminal::new(TestBackend::new(6, 2))?;
    draw_view(&mut clipped_terminal, &clipped)?;
    assert_eq!(cell_symbol(&clipped_terminal, 0, 0, 6), "┌");
    assert_eq!(cell_symbol(&clipped_terminal, 0, 1, 6), "│");
    assert!(symbol_position_opt(&clipped_terminal, "└", 6).is_none());

    Ok(())
}

/// Verifies code-block height saturates when source exceeds terminal row limits.
///
/// # Example Under Test
///
/// ```text
/// code_block("\n" repeated u16::MAX times)
/// terminal size = 4x1
/// ```
///
/// # Assertions
///
/// - The code block reports [`u16::MAX`] as its intrinsic height.
/// - Rendering the oversized source succeeds without arithmetic overflow.
///
/// # Why
///
/// Adding code-block borders to an already saturated content height must not
/// panic in debug builds or wrap in release builds.
#[test]
fn code_block_height_saturates_beyond_terminal_row_limit() -> Result<()> {
    let view = code_block("\n".repeat(usize::from(u16::MAX)));
    let mut terminal = Terminal::new(TestBackend::new(4, 1))?;
    let mut min_height = 0.0;
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        min_height = measure_view_in_area(&view, &mut ctx).height;
        render_result = view.render(&mut ctx);
    })?;
    render_result?;

    assert_eq!(min_height, f32::from(u16::MAX));

    Ok(())
}
