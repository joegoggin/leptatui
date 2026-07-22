/// Verifies semantic blocks and readable fallbacks retain source order.
///
/// # Example Under Test
///
/// ```text
/// # Start
///
/// ---
///
/// ![middle](middle.png)
///
/// <end>
/// ```
///
/// # Assertions
///
/// - The semantic heading remains first.
/// - Rule, image, and raw-HTML fallbacks retain their original order.
/// - Empty separator paragraphs retain one terminal row between fallbacks.
#[test]
fn markdown_preserves_fallback_source_order() {
    let source = "# Start\n\n---\n\n![middle](middle.png)\n\n<end>\n";

    assert_eq!(
        markdown(source),
        column(separate_blocks(views![
            h1("Start"),
            thematic_break(),
            paragraph("Image: middle (middle.png)"),
            paragraph(Text::from(vec![Line::raw("<end>"), Line::default()])),
        ])),
    );
}

/// Verifies Markdown conversion preserves mixed block source order.
///
/// # Example Under Test
///
/// ```text
/// # 開始
///
/// Before.
///
/// - 中
///
/// ## 終了
/// ```
///
/// # Assertions
///
/// - Parsing succeeds without reordering block types.
/// - Unicode content remains intact in headings and list items.
/// - Empty separator paragraphs retain one terminal row between blocks.
#[test]
fn markdown_preserves_source_order() {
    let source = "# 開始\n\nBefore.\n\n- 中\n\n## 終了\n";

    assert_eq!(
        markdown(source),
        column(separate_blocks(views![
            h1("開始"),
            paragraph("Before."),
            unordered_list([list_item([paragraph("中")])]),
            h2("終了"),
        ])),
    );
}

/// Verifies empty Markdown produces an empty scrollable document.
///
/// # Example Under Test
///
/// ```text
/// source = ""
/// ```
///
/// # Assertions
///
/// - Parsing succeeds without a fallible API.
/// - The result is an empty semantic column.
#[test]
fn markdown_empty_source_returns_empty_column() {
    assert_eq!(markdown(""), column(()));
}
