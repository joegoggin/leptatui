/// Verifies Markdown images become descriptive text without image loading.
///
/// # Example Under Test
///
/// ```text
/// ![diagram](https://example.com/diagram.png)
/// ![](local.png)
/// ![caption]()
/// ![]()
/// ```
///
/// # Assertions
///
/// - Alt text and source are both shown when present.
/// - Source-only and alt-only images remain descriptive.
/// - An image with neither value still has a readable label.
/// - Every image maps to text rather than a path-backed image view.
/// - Empty separator paragraphs retain one terminal row between images.
#[test]
fn markdown_maps_images_to_descriptive_text() {
    let source = concat!(
        "![diagram](https://example.com/diagram.png)\n\n",
        "![](local.png)\n\n",
        "![caption]()\n\n",
        "![]()\n",
    );

    assert_eq!(
        markdown(source),
        column(separate_blocks(views![
            paragraph("Image: diagram (https://example.com/diagram.png)"),
            paragraph("Image: local.png"),
            paragraph("Image: caption"),
            paragraph("Image"),
        ])),
    );
}

/// Verifies raw HTML remains literal and entities follow CommonMark parsing.
///
/// # Example Under Test
///
/// ```text
/// Before <kbd>&amp;</kbd> after.
///
/// <section>
/// block &amp;
/// </section>
///
/// Fish &amp; Chips &copy;
/// ```
///
/// # Assertions
///
/// - Inline HTML tags are retained around decoded CommonMark text.
/// - Block HTML tags, entities, and source line endings remain literal.
/// - Entities in ordinary Markdown text decode to their visible characters.
/// - Following semantic content remains in source order.
/// - Empty separator paragraphs retain one terminal row between blocks.
#[test]
fn markdown_preserves_literal_html_and_entities() {
    let source = concat!(
        "Before <kbd>&amp;</kbd> after.\n\n",
        "<section>\n",
        "block &amp;\n",
        "</section>\n\n",
        "Fish &amp; Chips &copy;\n",
    );

    assert_eq!(
        markdown(source),
        column(separate_blocks(views![
            paragraph("Before <kbd>&</kbd> after."),
            paragraph(Text::from(vec![
                Line::raw("<section>"),
                Line::raw("block &amp;"),
                Line::raw("</section>"),
                Line::default(),
            ])),
            paragraph("Fish & Chips ©"),
        ])),
    );
}

/// Verifies textual extension events remain readable when encountered.
///
/// # Example Under Test
///
/// ```text
/// - [x] ~~done~~ and $x + y$[^note]
///
/// $$z$$
///
/// [^note]: Detail
/// ```
///
/// # Assertions
///
/// - Unsupported inline presentation drops styling but retains its payload.
/// - Task and footnote events receive readable terminal markers.
/// - Display-math and footnote-definition text stays in source order.
/// - Production parsing remains limited to CommonMark plus tables.
/// - Empty separator paragraphs retain one terminal row between blocks.
#[test]
fn markdown_preserves_text_from_unsupported_parser_events() {
    let source = concat!(
        "- [x] ~~done~~ and $x + y$[^note]\n\n",
        "$$z$$\n\n",
        "[^note]: Detail\n",
    );
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_MATH
        | Options::ENABLE_FOOTNOTES;
    let mut parser = Parser::new_ext(source, options);
    let mut context = MarkdownParseContext::new(Path::new("."), None);

    assert_eq!(
        column(parse_blocks(
            &mut parser,
            None,
            MarkdownOptions::default(),
            &mut context,
        )),
        column(separate_blocks(views![
            unordered_list([list_item([paragraph("[x] done and x + y[^note]")])]),
            paragraph("z"),
            paragraph("Detail"),
        ])),
    );
}
