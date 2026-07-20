//! Markdown fixture integration tests.
//!
//! These tests exercise the public Markdown reader APIs against representative
//! documents and verify both semantic view construction and stable fragments of
//! terminal-buffer output.

use leptatui::{
    AnyView, Borders, CellAlignment, CodeBlockView, IntoView, IntoViews, LayoutView,
    MarkdownOptions, Modifier, RenderCtx, Result, SyntaxTheme, TuiSpacing, TuiStyle, View, block,
    code_block, column, h1, h2, h3, h4, h5, h6, list_item, markdown, markdown_with_options,
    ordered_list, paragraph, table, table_body, table_cell, table_head, table_row, unordered_list,
    view::{Line, Span, Text},
};
use ratatui::{Terminal, backend::TestBackend, style::Style};

mod support;

use support::{render_view, rendered_lines};

/// Representative headings, paragraphs, inline syntax, and list fixture.
const CORE_FIXTURE: &str = include_str!("fixtures/markdown/core.md");
/// Representative readable fallback and table fixture.
const FALLBACKS_FIXTURE: &str = include_str!("fixtures/markdown/fallbacks.md");
/// Representative fenced, indented, highlighted, and fallback code fixture.
const CODE_FIXTURE: &str = include_str!("fixtures/markdown/code.md");
/// Zero-content Markdown fixture.
const EMPTY_FIXTURE: &str = include_str!("fixtures/markdown/empty.md");

/// Creates the expected semantic sequence for visibly separated Markdown blocks.
///
/// # Arguments
///
/// * `blocks` — Content-bearing Markdown blocks in source order.
///
/// # Returns
///
/// A [`Vec`] containing one empty paragraph between each block.
fn separated_blocks(blocks: impl IntoViews) -> Vec<AnyView> {
    let blocks = blocks.into_views();
    let mut separated = Vec::with_capacity(blocks.len().saturating_mul(2).saturating_sub(1));

    for block in blocks {
        if !separated.is_empty() {
            separated.push(paragraph("").into_view());
        }
        separated.push(block);
    }

    separated
}

/// Creates the semantic blockquote fallback used by Markdown conversion.
///
/// # Arguments
///
/// * `children` — Semantic children nested inside the quote.
///
/// # Returns
///
/// A left-bordered [`View`] matching the public Markdown presentation.
fn block_quote(children: impl IntoViews) -> impl View {
    block(column(children)).with_inline_style(
        TuiStyle::new()
            .borders(Borders::LEFT)
            .padding(TuiSpacing::new(1, 0, 0, 0)),
    )
}

/// Creates the semantic thematic-break fallback used by Markdown conversion.
///
/// # Returns
///
/// A top-bordered [`View`] matching the public Markdown presentation.
fn thematic_break() -> impl View {
    block(column(())).with_inline_style(TuiStyle::new().borders(Borders::TOP))
}

/// Asserts two view trees produce the same terminal output.
fn assert_views_render_equally(actual: &AnyView, expected: &dyn View) {
    let actual = render_view(actual.as_view(), 80, 200).expect("actual view should render");
    let expected = render_view(expected, 80, 200).expect("expected view should render");

    assert_eq!(rendered_lines(&actual), rendered_lines(&expected));
}

/// Verifies the core fixture maps CommonMark structure into semantic views.
///
/// # Example Under Test
///
/// ```text
/// tests/fixtures/markdown/core.md
/// ```
///
/// # Assertions
///
/// - H1 through H6 retain their levels and source order.
/// - Unicode paragraph content and nested inline modifiers remain intact.
/// - Link labels are underlined and display a readable destination.
/// - Mixed nested lists retain loose paragraphs, empty items, and non-one starts.
/// - Empty separator paragraphs retain one terminal row between blocks.
#[test]
fn markdown_core_fixture_builds_semantic_views() {
    let italic = Style::new().add_modifier(Modifier::ITALIC);
    let underline = Style::new().add_modifier(Modifier::UNDERLINED);

    let actual = markdown(CORE_FIXTURE);
    let expected = column(separated_blocks((
        h1("One"),
        h2("Two"),
        h3("Three"),
        h4("Four"),
        h5("Five"),
        h6("Six"),
        paragraph(
            "This paragraph is deliberately long enough to wrap in a narrow terminal while preserving Unicode 界 characters.",
        ),
        paragraph(Text::from(Line::from(vec![
            Span::styled("outer ", italic),
            Span::styled("bold 界", italic.add_modifier(Modifier::BOLD)),
            Span::styled(" tail", italic),
            Span::raw(" and "),
            Span::styled("plain", Style::new().add_modifier(Modifier::BOLD)),
            Span::raw(" with "),
            Span::styled("code", Style::new().add_modifier(Modifier::REVERSED)),
            Span::raw(" plus "),
            Span::styled("the guide (https://example.com/guide)", underline),
            Span::raw("."),
        ]))),
        ordered_list([
            list_item(separated_blocks((
                paragraph("First"),
                paragraph("Second paragraph."),
                unordered_list([
                    list_item(separated_blocks((
                        paragraph("Nested bullet"),
                        ordered_list([list_item([paragraph("Nested number")])]).start(7),
                    ))),
                    list_item(()),
                ]),
            ))),
            list_item([paragraph("Last")]),
        ])
        .start(3),
    )));
    assert_views_render_equally(&actual, &expected);
}

/// Verifies fallback fixture blocks remain readable and semantically ordered.
///
/// # Example Under Test
///
/// ```text
/// tests/fixtures/markdown/fallbacks.md
/// ```
///
/// # Assertions
///
/// - Nested blockquotes and thematic rules use visible fallback blocks.
/// - Images and literal HTML become deterministic paragraph content.
/// - Table sections and cell alignments match the Markdown delimiter row.
/// - Malformed-looking but parseable delimiters remain literal text.
/// - Empty separator paragraphs retain one terminal row between blocks.
#[test]
fn markdown_fallback_fixture_builds_semantic_views() {
    let aligned_cells = |values: [&'static str; 4]| {
        [
            table_cell(values[0]).alignment(CellAlignment::Left),
            table_cell(values[1]).alignment(CellAlignment::Left),
            table_cell(values[2]).alignment(CellAlignment::Center),
            table_cell(values[3]).alignment(CellAlignment::Right),
        ]
    };

    let actual = markdown(FALLBACKS_FIXTURE);
    let expected = column(separated_blocks((
        block_quote(separated_blocks((
            paragraph("Alpha beta gamma"),
            block_quote([paragraph("Inner")]),
        ))),
        thematic_break(),
        paragraph("Image: diagram (https://example.com/diagram.png)"),
        paragraph("Before <kbd>&</kbd> after."),
        paragraph(Text::from(vec![
            Line::raw("<section>"),
            Line::raw("literal &amp;"),
            Line::raw("</section>"),
            Line::default(),
        ])),
        table([
            table_head([table_row(aligned_cells([
                "Default", "Left", "Center", "Right",
            ]))]),
            table_body([table_row(aligned_cells([
                "alpha", "beta", "gamma", "delta",
            ]))]),
        ]),
        paragraph("Unclosed **strong and [link](https://example.com"),
    )));
    assert_views_render_equally(&actual, &expected);
}

/// Verifies code fixtures preserve fence selection and fallback behavior.
///
/// # Example Under Test
///
/// ```text
/// tests/fixtures/markdown/code.md
/// ```
///
/// # Assertions
///
/// - Fenced code uses only the first info-string token.
/// - The `rust` token and `rs` alias retain highlighted semantic code blocks.
/// - Unknown languages remain labeled while falling back to plain source.
/// - Empty, indented, long, and Unicode code sources remain intact.
/// - Empty separator paragraphs retain one terminal row between code blocks.
#[test]
fn markdown_code_fixture_builds_semantic_views() {
    let actual = markdown(CODE_FIXTURE);
    let expected = column(separated_blocks([
        code_block("fn main() {\n    println!(\"界\");\n}\n").language("rust"),
        code_block("let value = true;\n").language("rs"),
        code_block("plain\n").language("unknown-language"),
        code_block(""),
        code_block("indented 界\n"),
        code_block("abcdefghijklmnopqrstuvwxyz界\n").language("text"),
    ]));
    assert_views_render_equally(&actual, &expected);
}

/// Verifies fixture documents render stable terminal fragments without snapshots.
///
/// # Example Under Test
///
/// ```text
/// core.md at 24x80
/// fallbacks.md at 48x80
/// ```
///
/// # Assertions
///
/// - Markdown H1 through H6 use the same repeated `#` heading hierarchy.
/// - Long Unicode prose wraps across multiple terminal rows.
/// - Ordered and unordered list markers remain visible.
/// - Links expose their destinations in terminal text.
/// - Quote prefixes, rules, image fallbacks, literal HTML, and tables render visibly.
#[test]
fn markdown_fixtures_render_targeted_terminal_fragments() -> Result<()> {
    let core_document = markdown(CORE_FIXTURE);
    let core = render_view(core_document.as_view(), 24, 80)?;
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
    assert!(
        core_lines
            .iter()
            .any(|line| line.contains("https://example.com"))
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
        .downcast_ref::<LayoutView>()
        .expect("Markdown document should be a column layout");
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
            .downcast_ref::<LayoutView>()
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
