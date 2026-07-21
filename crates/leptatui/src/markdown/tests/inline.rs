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

/// Verifies Markdown links remain readable without terminal link interaction.
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
/// - Link labels are underlined and retain nested emphasis.
/// - A descriptive label is followed by its parenthesized destination.
/// - URL labels and URL autolinks do not duplicate their destinations.
/// - Email autolinks do not expose or duplicate the `mailto:` scheme.
/// - Links with empty destinations do not display empty parentheses.
#[test]
fn markdown_styles_links_and_appends_hidden_destinations() {
    let source = concat!(
        "Read [the *guide*](https://example.com/guide), ",
        "[https://example.com](https://example.com), ",
        "<https://example.org>, and <reader@example.com>, plus [empty]().\n",
    );
    let underline = Style::new().add_modifier(Modifier::UNDERLINED);

    assert_eq!(
        markdown(source),
        column([paragraph(Text::from(Line::from(vec![
            Span::raw("Read "),
            Span::styled("the ", underline),
            Span::styled("guide", underline.add_modifier(Modifier::ITALIC),),
            Span::styled(" (https://example.com/guide)", underline),
            Span::raw(", "),
            Span::styled("https://example.com", underline),
            Span::raw(", "),
            Span::styled("https://example.org", underline),
            Span::raw(", and "),
            Span::styled("reader@example.com", underline),
            Span::raw(", plus "),
            Span::styled("empty", underline),
            Span::raw("."),
        ])))]),
    );
}
