/// Verifies Markdown lists retain starts, blocks, nesting, and empty items.
///
/// # Example Under Test
///
/// ```text
/// 3. First
///
///    Second paragraph.
///
///    - Nested bullet
///
///      7. Nested number
///    -
/// 4. Last
/// ```
///
/// # Assertions
///
/// - The outer ordered list starts at three.
/// - Loose item paragraphs remain separate blocks.
/// - Mixed ordered and unordered nesting retains its hierarchy.
/// - Tight-list text becomes paragraphs and empty items remain present.
/// - Empty separator paragraphs retain block spacing inside loose items.
#[test]
fn markdown_preserves_nested_and_mixed_lists() {
    let source = concat!(
        "3. First\n\n",
        "   Second paragraph.\n\n",
        "   - Nested bullet\n\n",
        "     7. Nested number\n",
        "   -\n",
        "4. Last\n",
    );

    assert_eq!(
        markdown(source),
        div([ordered_list([
            list_item(separate_blocks(views![
                paragraph("First"),
                paragraph("Second paragraph."),
                unordered_list([
                    list_item(separate_blocks(views![
                        paragraph("Nested bullet"),
                        ordered_list([list_item([paragraph("Nested number")])]).start(7),
                    ])),
                    list_item(()),
                ]),
            ])),
            list_item([paragraph("Last")]),
        ])
        .start(3)]),
    );
}

/// Verifies Markdown tables map sections, rows, cells, and alignments.
///
/// # Example Under Test
///
/// ```text
/// | Default | Left | Center | Right |
/// | ------- | :--- | :----: | ----: |
/// | a       | b    | c      | d     |
/// ```
///
/// # Assertions
///
/// - Parsing succeeds with the table extension enabled.
/// - Header cells are wrapped in a synthesized semantic header row.
/// - Body rows remain in the semantic table body.
/// - Default, left, center, and right alignments map to semantic cells.
#[test]
fn markdown_maps_table_structure_and_alignment() {
    let source = concat!(
        "| Default | Left | Center | Right |\n",
        "| ------- | :--- | :----: | ----: |\n",
        "| a       | b    | c      | d     |\n",
    );
    let aligned_cells = |values: [&'static str; 4]| {
        [
            table_cell(values[0]).alignment(CellAlignment::Left),
            table_cell(values[1]).alignment(CellAlignment::Left),
            table_cell(values[2]).alignment(CellAlignment::Center),
            table_cell(values[3]).alignment(CellAlignment::Right),
        ]
    };

    assert_eq!(
        markdown(source),
        div([table([
            table_head([table_row(aligned_cells([
                "Default", "Left", "Center", "Right",
            ]))]),
            table_body([table_row(aligned_cells(["a", "b", "c", "d"]))]),
        ])]),
    );
}

/// Verifies nested blockquotes retain semantic children and visible prefixes.
///
/// # Example Under Test
///
/// ```text
/// > Alpha beta gamma
/// >
/// > > Inner
/// ```
///
/// # Assertions
///
/// - Each quote becomes a left-bordered block with readable padding.
/// - The outer border remains visible beside wrapped content.
/// - The nested quote stacks a second border without flattening its child.
/// - The blank row between quote blocks retains the outer quote border.
#[test]
fn markdown_renders_nested_blockquotes_with_wrapped_prefixes() -> Result<()> {
    let source = "> Alpha beta gamma\n>\n> > Inner\n";

    assert_eq!(
        markdown(source),
        div([block_quote(views![
            paragraph("Alpha beta gamma"),
            paragraph(""),
            block_quote(views![paragraph("Inner")]),
        ])]),
    );

    let lines = rendered_markdown_lines(source, 12, 4)?;
    assert!(lines[0].starts_with("│ Alpha beta"));
    assert!(lines[1].starts_with("│ gamma"));
    assert_eq!(lines[2].trim_end(), "│");
    assert!(lines[3].starts_with("│ │ Inner"));

    Ok(())
}

/// Verifies thematic breaks render as width-responsive terminal rules.
///
/// # Example Under Test
///
/// ```text
/// ---
/// ```
///
/// # Assertions
///
/// - A thematic break maps to a dedicated one-row fallback block.
/// - A one-cell terminal renders one horizontal rule glyph without panic.
/// - Wider terminals fill the complete row with rule glyphs.
#[test]
fn markdown_renders_thematic_breaks_at_narrow_widths() -> Result<()> {
    assert_eq!(markdown("---\n"), div([thematic_break()]));
    assert_eq!(rendered_markdown_lines("---\n", 1, 1)?, ["─"]);
    assert_eq!(rendered_markdown_lines("---\n", 6, 1)?, ["──────"]);

    Ok(())
}
