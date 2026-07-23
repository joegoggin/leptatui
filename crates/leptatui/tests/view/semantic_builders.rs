/// Verifies semantic list builders store items, starts, and selector metadata.
///
/// # Example Under Test
///
/// ```text
/// ordered_list([list_item([])]).start(3)
/// unordered_list([list_item([paragraph("Body")])])
/// ```
///
/// # Assertions
///
/// - Ordered lists retain their items, configured start, and ordered-list type.
/// - Unordered lists retain their items and unordered-list type.
/// - List items retain block children and list-item type.
#[test]
fn semantic_list_builders_store_items_starts_and_metadata() {
    let ordered = ordered_list([list_item(())]).start(3);
    assert_eq!(ordered.start_value(), 3);
    assert_eq!(ordered.metadata().view_type(), ViewType::OrderedList);
    assert_eq!(ordered.children().len(), 1);
    let item = ordered.children()[0]
        .downcast_ref::<ListItemView>()
        .expect("expected list-item view");
    assert!(item.children().is_empty());
    assert_eq!(item.metadata().view_type(), ViewType::ListItem);

    let unordered = unordered_list([list_item([paragraph("Body")])]);
    assert_eq!(unordered.metadata().view_type(), ViewType::UnorderedList);
    assert_eq!(unordered.children().len(), 1);
}

/// Verifies semantic table builders store structure, rich text, and alignment.
///
/// # Example Under Test
///
/// ```text
/// table([table_head([table_row([table_cell("Name")])])])
/// table_cell(rich_text).alignment(CellAlignment::Right)
/// ```
///
/// # Assertions
///
/// - Every builder stores its corresponding semantic view type.
/// - Table cells retain rich text.
/// - Cells default to left alignment and accept an explicit alignment.
#[test]
fn semantic_table_builders_store_structure_content_and_alignment() {
    let rich = Text::from(Line::from(vec![
        Span::raw("Na"),
        Span::styled("me", Style::new().fg(Color::Yellow)),
    ]));
    let view = table([table_head([table_row([table_cell(rich.clone())])])]);
    assert_eq!(view.metadata().view_type(), ViewType::Table);
    let head = view.children()[0]
        .downcast_ref::<TableSectionView>()
        .expect("expected table-head view");
    assert_eq!(head.metadata().view_type(), ViewType::TableHead);
    let row = head.children()[0]
        .downcast_ref::<TableRowView>()
        .expect("expected table-row view");
    assert_eq!(row.metadata().view_type(), ViewType::TableRow);
    let cell = row.children()[0]
        .downcast_ref::<TableCellView>()
        .expect("expected table-cell view");
    assert_eq!(cell.content(), &rich);
    assert_eq!(cell.cell_alignment(), CellAlignment::Left);
    assert_eq!(cell.metadata().view_type(), ViewType::TableCell);

    let aligned = table_cell("Ready").alignment(CellAlignment::Right);
    assert_eq!(aligned.cell_alignment(), CellAlignment::Right);

    let body = table_body(());
    assert_eq!(body.metadata().view_type(), ViewType::TableBody);
}

/// Verifies semantic builders retain standard Ratatui text conversions.
///
/// # Example Under Test
///
/// ```text
/// h1(Vec<Line>)
/// paragraph(Cow<str>)
/// table_cell(&[Line])
/// ```
///
/// # Assertions
///
/// - Headings retain multi-line `Vec<Line>` content.
/// - Paragraphs retain borrowed string content.
/// - Table cells retain borrowed line-slice content.
#[test]
fn semantic_builders_accept_text_compatible_inputs() {
    let heading = h1(vec![Line::raw("First"), Line::raw("Second")]);
    let paragraph = paragraph(Cow::Borrowed("Borrowed"));
    let cell_lines = [Line::raw("Cell one"), Line::raw("Cell two")];
    let cell = table_cell(cell_lines.as_slice());

    assert_eq!(heading.content().to_string(), "First\nSecond");
    assert_eq!(paragraph.content().to_string(), "Borrowed");
    assert_eq!(cell.content().to_string(), "Cell one\nCell two");
}
