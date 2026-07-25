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
    let expected = div(separated_blocks((
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
