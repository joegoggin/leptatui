/// Verifies Markdown inline syntax produces composable terminal modifiers.
///
/// # Example Under Test
///
/// ```text
/// *outer **bold 界** tail* and **plain** with `code` plus \*escaped\*.
/// ```
///
/// # Assertions
///
/// - Emphasis uses italics and strong text uses bold.
/// - Nested emphasis and strong text combine both modifiers.
/// - Inline code uses reverse video without changing its content.
/// - Escaped delimiters remain literal unstyled text.
/// - Adjacent parser text events coalesce into stable spans.
#[test]
fn markdown_styles_nested_inline_syntax_and_escaped_text() {
    let source = "*outer **bold 界** tail* and **plain** with `code` plus \\*escaped\\*.\n";

    assert_eq!(
        markdown(source),
        column([paragraph(Text::from(Line::from(vec![
            Span::styled("outer ", Style::new().add_modifier(Modifier::ITALIC),),
            Span::styled(
                "bold 界",
                Style::new().add_modifier(Modifier::ITALIC | Modifier::BOLD),
            ),
            Span::styled(" tail", Style::new().add_modifier(Modifier::ITALIC),),
            Span::raw(" and "),
            Span::styled("plain", Style::new().add_modifier(Modifier::BOLD)),
            Span::raw(" with "),
            Span::styled("code", Style::new().add_modifier(Modifier::REVERSED)),
            Span::raw(" plus *escaped*.")
        ])))]),
    );
}

/// Verifies Markdown links retain readable labels and focusable metadata.
///
/// # Example Under Test
///
/// ```text
/// Read [the *guide*](https://example.com/guide),
/// [https://example.com](https://example.com), <https://example.org>,
/// and <reader@example.com>, plus [empty]().
/// ```
///
/// # Assertions
///
/// - Link labels retain nested emphasis without appended destinations.
/// - URL and email autolinks retain their readable labels.
/// - Actionable targets participate in focus traversal.
/// - Links with empty destinations remain inactive.
#[test]
fn markdown_links_retain_labels_and_focusable_metadata() {
    let source = concat!(
        "Read [the *guide*](https://example.com/guide), ",
        "[https://example.com](https://example.com), ",
        "<https://example.org>, and <reader@example.com>, plus [empty]().\n",
    );
    let actual = markdown(source);
    assert_eq!(actual.__focusable_count(), 4);
    let document = actual
        .downcast_ref::<LayoutView>()
        .expect("Markdown document should be a column layout");
    let [paragraph] = document.children() else {
        panic!("expected one linked paragraph");
    };
    let paragraph = paragraph
        .downcast_ref::<ParagraphView>()
        .expect("Markdown child should be a paragraph");
    assert_eq!(
        paragraph.content(),
        &Text::from(Line::from(vec![
            Span::raw("Read "),
            Span::raw("the "),
            Span::styled("guide", Style::new().add_modifier(Modifier::ITALIC)),
            Span::raw(", "),
            Span::raw("https://example.com"),
            Span::raw(", "),
            Span::raw("https://example.org"),
            Span::raw(", and "),
            Span::raw("reader@example.com"),
            Span::raw(", plus "),
            Span::raw("empty"),
            Span::raw("."),
        ])),
    );
}
