/// Verifies table headers, borders, and cell alignments render semantically.
///
/// # Example Under Test
///
/// ```text
/// table([
///     table_head([table_row(["Name", "Status"])]),
///     table_body([table_row([center "A", right "OK"])]),
/// ])
/// terminal size = 13x5
/// ```
///
/// # Assertions
///
/// - Plain outer borders and intersections frame both rows.
/// - Header text uses the bold semantic default.
/// - Centered and right-aligned cells use their allocated column widths.
#[test]
fn semantic_table_renders_borders_bold_header_and_alignment() -> leptatui::app::Result<()> {
    let view = table([
        table_head([table_row([table_cell("Name"), table_cell("Status")])]),
        table_body([table_row([
            table_cell("A").alignment(CellAlignment::Center),
            table_cell("OK").alignment(CellAlignment::Right),
        ])]),
    ]);
    let mut terminal = Terminal::new(TestBackend::new(13, 5))?;

    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 0, 0, 13), "┌");
    assert_eq!(cell_symbol(&terminal, 5, 0, 13), "┬");
    assert_eq!(cell_symbol(&terminal, 12, 4, 13), "┘");
    assert!(cell_modifiers(&terminal, 1, 1, 13).contains(Modifier::BOLD));
    assert_eq!(symbol_position(&terminal, "A", 13), (3, 3));
    assert_eq!(symbol_position(&terminal, "O", 13), (10, 3));

    Ok(())
}

/// Verifies table selector styles cascade through sections and cells.
///
/// # Example Under Test
///
/// ```text
/// TableHead { modifier: empty }
/// .status { fg: Green }
/// table_head([table_row([table_cell("Ready").with_classes("status")])])
/// ```
///
/// # Assertions
///
/// - An authored table-head type rule removes the bold semantic default.
/// - A table-cell class rule colors its rich text content.
#[test]
fn semantic_table_styles_override_header_defaults_and_style_cells() -> leptatui::app::Result<()> {
    let view = table([table_head([table_row([
        table_cell("Ready").with_classes("status")
    ])])]);
    let stylesheet = Stylesheet::new()
        .rule(
            StyleSelector::view_type(ViewType::TableHead),
            TuiStyle::new().modifier(Modifier::empty()),
        )
        .rule(
            StyleSelector::class("status"),
            TuiStyle::new().foreground(Color::Green),
        );
    let mut terminal = Terminal::new(TestBackend::new(7, 3))?;
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert!(!cell_modifiers(&terminal, 1, 1, 7).contains(Modifier::BOLD));
    assert_eq!(cell_colors(&terminal, 1, 1, 7).0, Color::Green);

    Ok(())
}

/// Verifies table background styles fill populated and normalized cells.
///
/// # Example Under Test
///
/// ```text
/// table([div(["A"]), div(["B", "C"])]).background(Blue)
/// ```
///
/// # Assertions
///
/// - A populated cell retains the table background.
/// - A normalized empty trailing cell retains the table background.
///
/// # Why
///
/// Background colors are surface styles rather than inherited text styles, so
/// the table renderer must paint the grid area before rendering its cells.
#[test]
fn semantic_table_background_fills_grid_cells() -> leptatui::app::Result<()> {
    let view = table([table_body([
        table_row([table_cell("A")]),
        table_row([table_cell("B"), table_cell("C")]),
    ])])
    .with_inline_style(TuiStyle::new().background(Color::Blue));
    let mut terminal = Terminal::new(TestBackend::new(5, 5))?;

    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_colors(&terminal, 1, 1, 5).1, Color::Blue);
    assert_eq!(cell_colors(&terminal, 3, 1, 5).1, Color::Blue);

    Ok(())
}

/// Verifies table section, row, and cell backgrounds use structural precedence.
///
/// # Example Under Test
///
/// ```text
/// TableHead { bg: Blue }
/// .accent-row { bg: Green }
/// .accent-cell { bg: Yellow }
/// ```
///
/// # Assertions
///
/// - Header cells retain the table-head background.
/// - Body cells retain an explicit table-row background.
/// - An explicit table-cell background overrides its row background.
///
/// # Why
///
/// Backgrounds are surface styles rather than inherited text styles, so table
/// layout must retain them while flattening sections and rows for rendering.
#[test]
fn semantic_table_structural_backgrounds_respect_precedence() -> leptatui::app::Result<()> {
    let view = table([
        table_head([table_row([table_cell("H"), table_cell("I")])]),
        table_body([
            table_row([table_cell("A"), table_cell("B").with_classes("accent-cell")])
                .with_classes("accent-row"),
        ]),
    ]);
    let stylesheet = Stylesheet::new()
        .rule(
            StyleSelector::view_type(ViewType::TableHead),
            TuiStyle::new().background(Color::Blue),
        )
        .rule(
            StyleSelector::class("accent-row"),
            TuiStyle::new().background(Color::Green),
        )
        .rule(
            StyleSelector::class("accent-cell"),
            TuiStyle::new().background(Color::Yellow),
        );
    let mut terminal = Terminal::new(TestBackend::new(5, 5))?;
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = ctx.__with_stylesheet(&stylesheet, |ctx| view.render(ctx));
    })?;
    render_result?;

    assert_eq!(cell_colors(&terminal, 1, 1, 5).1, Color::Blue);
    assert_eq!(cell_colors(&terminal, 3, 1, 5).1, Color::Blue);
    assert_eq!(cell_colors(&terminal, 1, 3, 5).1, Color::Green);
    assert_eq!(cell_colors(&terminal, 3, 3, 5).1, Color::Yellow);

    Ok(())
}

/// Verifies content-aware columns shrink and wrapped cells determine row height.
///
/// # Example Under Test
///
/// ```text
/// table_body([table_row(["abcdef", "界界"])])
/// terminal size = 9x4
/// ```
///
/// # Assertions
///
/// - Both preferred columns shrink to three content cells.
/// - ASCII and double-width Unicode content wrap to a second row.
/// - The bottom border follows the tallest wrapped cell.
#[test]
fn semantic_table_shrinks_columns_and_wraps_unicode_cells() -> leptatui::app::Result<()> {
    let view = table([table_body([table_row([
        table_cell("abcdef"),
        table_cell("界界"),
    ])])]);
    let mut terminal = Terminal::new(TestBackend::new(9, 4))?;
    let mut min_height = 0.0;
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        min_height = measure_view_in_area(&view, &mut ctx).height;
    })?;

    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 0, 0, 9), "┌");
    assert_eq!(cell_symbol(&terminal, 4, 0, 9), "┬");
    assert_eq!(cell_symbol(&terminal, 8, 0, 9), "┐");
    assert_eq!(symbol_position(&terminal, "a", 9), (1, 1));
    assert_eq!(symbol_position(&terminal, "d", 9), (1, 2));
    assert_eq!(cell_symbol(&terminal, 5, 1, 9), "界");
    assert_eq!(cell_symbol(&terminal, 5, 2, 9), "界");
    assert_eq!(cell_symbol(&terminal, 0, 3, 9), "└");
    assert_eq!(min_height, 4.0);

    Ok(())
}

/// Verifies uneven rows normalize to the widest row's column count.
///
/// # Example Under Test
///
/// ```text
/// table_body([div(["A"]), div(["B", "C", "D"])])
/// terminal size = 7x5
/// ```
///
/// # Assertions
///
/// - The first row receives two empty trailing cells.
/// - The extra cells in the second row expand the shared grid to three columns.
/// - All vertical separators remain aligned between rows.
#[test]
fn semantic_table_normalizes_missing_and_extra_cells() -> leptatui::app::Result<()> {
    let view = table([table_body([
        table_row([table_cell("A")]),
        table_row([table_cell("B"), table_cell("C"), table_cell("D")]),
    ])]);
    let mut terminal = Terminal::new(TestBackend::new(7, 5))?;

    draw_view(&mut terminal, &view)?;

    assert_eq!(cell_symbol(&terminal, 0, 0, 7), "┌");
    assert_eq!(cell_symbol(&terminal, 2, 0, 7), "┬");
    assert_eq!(cell_symbol(&terminal, 4, 0, 7), "┬");
    assert_eq!(cell_symbol(&terminal, 6, 0, 7), "┐");
    assert_eq!(cell_symbol(&terminal, 2, 1, 7), "│");
    assert_eq!(cell_symbol(&terminal, 4, 1, 7), "│");
    assert_eq!(cell_symbol(&terminal, 6, 1, 7), "│");
    assert_eq!(symbol_position(&terminal, "D", 7), (5, 3));

    Ok(())
}

/// Verifies narrow tables clip only columns that cannot receive content width.
///
/// # Example Under Test
///
/// ```text
/// table_body([div(["A", "B", "C"])])
/// terminal widths = 5, 0, 1, and 2
/// ```
///
/// # Assertions
///
/// - A five-cell viewport retains two one-cell columns and their borders.
/// - The trailing third column is clipped.
/// - Viewports too narrow for one bordered content cell render and measure
///   without panicking.
#[test]
fn semantic_table_clips_columns_and_handles_zero_width() -> leptatui::app::Result<()> {
    let view = table([table_body([table_row([
        table_cell("A"),
        table_cell("B"),
        table_cell("C"),
    ])])]);
    let mut narrow = Terminal::new(TestBackend::new(5, 3))?;
    draw_view(&mut narrow, &view)?;
    assert_eq!(cell_symbol(&narrow, 0, 0, 5), "┌");
    assert_eq!(cell_symbol(&narrow, 2, 0, 5), "┬");
    assert_eq!(cell_symbol(&narrow, 4, 0, 5), "┐");
    assert!(symbol_position_opt(&narrow, "C", 5).is_none());

    for width in 0..=2 {
        let mut terminal = Terminal::new(TestBackend::new(width, 1))?;
        let mut min_height = 1.0;
        let mut render_result = Ok(());
        terminal.draw(|frame| {
            let mut ctx = RenderCtx::new(frame);
            min_height = measure_view_in_area(&view, &mut ctx).height;
            render_result = view.render(&mut ctx);
        })?;
        render_result?;
        assert_eq!(min_height, 0.0);
    }

    Ok(())
}

/// Verifies tables report document height and clip vertically at the viewport.
///
/// # Example Under Test
///
/// ```text
/// div([table([div(["A"])]), text("End")])
/// terminal size = 3x4
/// table([div(["abcdef"])]) in a 3x2 terminal
/// ```
///
/// # Assertions
///
/// - A following document block begins after the table's bottom border.
/// - A short viewport renders only the visible top border and first content row.
/// - Vertical clipping does not render a partial bottom boundary or panic.
#[test]
fn semantic_table_reserves_document_height_and_clips_vertically() -> leptatui::app::Result<()> {
    let document = div((
        table([table_body([table_row([table_cell("A")])])]),
        text("End"),
    ));
    let mut document_terminal = Terminal::new(TestBackend::new(3, 4))?;
    draw_view(&mut document_terminal, &document)?;
    assert_eq!(symbol_position(&document_terminal, "E", 3), (0, 3));

    let clipped = table([table_body([table_row([table_cell("abcdef")])])]);
    let mut clipped_terminal = Terminal::new(TestBackend::new(3, 2))?;
    draw_view(&mut clipped_terminal, &clipped)?;
    assert_eq!(cell_symbol(&clipped_terminal, 0, 0, 3), "┌");
    assert_eq!(cell_symbol(&clipped_terminal, 0, 1, 3), "│");
    assert!(symbol_position_opt(&clipped_terminal, "└", 3).is_none());

    Ok(())
}
