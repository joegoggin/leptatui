/// Verifies ordered markers use configured values and align across digit widths.
///
/// # Example Under Test
///
/// ```text
/// ordered_list(["Nine", "Ten"]).start(9)
/// terminal size = 12x2
/// ```
///
/// # Assertions
///
/// - Markers render as `9.` and `10.` beginning at the requested value.
/// - The shorter marker is right-aligned to the longer marker.
/// - Both item contents begin in the same terminal column.
#[test]
fn ordered_list_starts_and_aligns_multi_digit_markers() -> Result<()> {
    let view = ordered_list([
        list_item([paragraph("Nine")]),
        list_item([paragraph("Ten")]),
    ])
    .start(9);
    let mut terminal = Terminal::new(TestBackend::new(12, 2))?;

    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 0, 0, 12), " ");
    assert_eq!(cell_symbol(&terminal, 1, 0, 12), "9");
    assert_eq!(cell_symbol(&terminal, 2, 0, 12), ".");
    assert_eq!(cell_symbol(&terminal, 0, 1, 12), "1");
    assert_eq!(cell_symbol(&terminal, 1, 1, 12), "0");
    assert_eq!(cell_symbol(&terminal, 2, 1, 12), ".");
    assert_eq!(symbol_position(&terminal, "N", 12), (4, 0));
    assert_eq!(symbol_position(&terminal, "T", 12), (4, 1));

    Ok(())
}

/// Verifies list blocks wrap beneath content and preserve empty marker rows.
///
/// # Example Under Test
///
/// ```text
/// unordered_list([
///     list_item([paragraph("Alpha Beta"), paragraph("Tail")]),
///     list_item([]),
/// ])
/// terminal size = 8x4
/// ```
///
/// # Assertions
///
/// - Unordered items render `-` markers.
/// - Wrapped continuation text and later blocks align with item content.
/// - An empty item still consumes one marker row.
#[test]
fn unordered_list_wraps_mixed_blocks_and_renders_empty_items() -> Result<()> {
    let view = unordered_list([
        list_item([paragraph("Alpha Beta"), paragraph("Tail")]),
        list_item(()),
    ]);
    let mut terminal = Terminal::new(TestBackend::new(8, 4))?;

    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 0, 0, 8), "-");
    assert_eq!(symbol_position(&terminal, "A", 8), (2, 0));
    assert_eq!(symbol_position(&terminal, "B", 8), (2, 1));
    assert_eq!(symbol_position(&terminal, "T", 8), (2, 2));
    assert_eq!(cell_symbol(&terminal, 0, 3, 8), "-");

    Ok(())
}

/// Verifies recursive lists indent by two cells at every nesting level.
///
/// # Example Under Test
///
/// ```text
/// ordered_list([list_item([
///     paragraph("Parent"),
///     unordered_list([list_item([
///         paragraph("Child"),
///         ordered_list([list_item([paragraph("Grandchild")])]),
///     ])]),
/// ])])
/// ```
///
/// # Assertions
///
/// - The parent ordered marker starts at column zero.
/// - The nested unordered marker starts at column two.
/// - The recursively nested ordered marker starts at column four.
/// - Nested content renders after each local marker gutter.
#[test]
fn nested_lists_indent_two_cells_per_level() -> Result<()> {
    let view = ordered_list([list_item((
        paragraph("Parent"),
        unordered_list([list_item((
            paragraph("Child"),
            ordered_list([list_item([paragraph("Grandchild")])]),
        ))]),
    ))]);
    let mut terminal = Terminal::new(TestBackend::new(18, 3))?;

    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 0, 0, 18), "1");
    assert_eq!(symbol_position(&terminal, "P", 18), (3, 0));
    assert_eq!(cell_symbol(&terminal, 2, 1, 18), "-");
    assert_eq!(symbol_position(&terminal, "C", 18), (4, 1));
    assert_eq!(cell_symbol(&terminal, 4, 2, 18), "1");
    assert_eq!(cell_symbol(&terminal, 5, 2, 18), ".");
    assert_eq!(symbol_position(&terminal, "G", 18), (7, 2));

    Ok(())
}

/// Verifies list measurement tolerates clipped and zero-width content areas.
///
/// # Example Under Test
///
/// ```text
/// unordered_list([list_item([paragraph("Wrapped")])]) in a 1x1 terminal
/// row([unordered_list(...), unordered_list(...)]) in a 1x1 terminal
/// ```
///
/// # Assertions
///
/// - A one-cell list renders its marker while clipping content.
/// - A row assigning zero width to one list renders without panicking.
/// - The narrow row reports a positive intrinsic height.
#[test]
fn semantic_lists_handle_narrow_and_zero_width_content() -> Result<()> {
    let mut narrow = Terminal::new(TestBackend::new(1, 1))?;
    draw_view(
        &mut narrow,
        &unordered_list([list_item([paragraph("Wrapped")])]),
    )?;
    assert_eq!(cell_symbol(&narrow, 0, 0, 1), "-");

    let split_view = row([
        unordered_list([list_item([paragraph("A")])]),
        unordered_list([list_item([paragraph("B")])]),
    ]);
    let mut split = Terminal::new(TestBackend::new(1, 1))?;
    let mut min_height = 0;
    split.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        min_height = split_view.__min_height(&mut ctx);
    })?;
    draw_view(&mut split, &split_view)?;
    assert!(min_height >= 1);

    Ok(())
}

/// Verifies list intrinsic height participates in parent overflow scrolling.
///
/// # Example Under Test
///
/// ```text
/// column([ordered_list(["First", "Second", "Third"])])
/// terminal size = 8x2
/// PageDown
/// ```
///
/// # Assertions
///
/// - The initial viewport clips the third list item.
/// - PageDown is handled by the parent column.
/// - The scrolled viewport reveals the third list item.
#[test]
fn semantic_list_height_scrolls_inside_parent_column() -> Result<()> {
    let mut view = column([ordered_list([
        list_item([paragraph("First")]),
        list_item([paragraph("Second")]),
        list_item([paragraph("Third")]),
    ])]);
    let mut terminal = Terminal::new(TestBackend::new(8, 2))?;

    draw_view(&mut terminal, &view)?;
    assert!(symbol_position_opt(&terminal, "T", 8).is_none());
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_view(&mut terminal, &view)?;
    assert!(symbol_position_opt(&terminal, "T", 8).is_some());

    Ok(())
}
