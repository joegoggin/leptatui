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
/// - Link labels retain visible text while their target becomes focusable metadata.
/// - Mixed nested lists retain loose paragraphs, empty items, and non-one starts.
/// - Empty separator paragraphs retain one terminal row between blocks.
#[test]
fn markdown_core_fixture_builds_semantic_views() {
    let italic = Style::new().add_modifier(Modifier::ITALIC);

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
            Span::raw("the guide"),
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
    assert_eq!(actual.__focusable_count(), 1);
    assert_views_render_equally(&actual, &expected);
}
