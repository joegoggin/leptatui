// Normal block formatting flow integration tests.

/// Verifies default block children stack in source order and fill the inline axis.
///
/// # Example Under Test
///
/// ```text
/// div(["A", "B", "C"])
/// terminal size: 8x4
/// ```
///
/// # Assertions
///
/// - The root block fills the eight-cell containing width.
/// - Each child receives an automatic eight-cell width.
/// - The children occupy consecutive rows in source order.
#[test]
fn block_children_stack_in_source_order_at_automatic_width() -> Result<()> {
    let root = div((text("A"), text("B"), text("C"))).into_view();

    let terminal = render_layout_root(&root, 8, 4)?;

    let root_geometry = root
        .style_metadata()
        .and_then(StyleMetadata::layout_geometry)
        .expect("root block geometry");
    assert_eq!(root_geometry.border_box, ratatui::layout::Rect::new(0, 0, 8, 3));

    let children = root
        .downcast_ref::<leptatui::DivView>()
        .expect("Div root")
        .child_views();
    for (index, child) in children.iter().enumerate() {
        let geometry = child
            .style_metadata()
            .and_then(StyleMetadata::layout_geometry)
            .expect("block child geometry");
        assert_eq!(
            geometry.border_box,
            ratatui::layout::Rect::new(0, u16::try_from(index).unwrap_or(u16::MAX), 8, 1)
        );
    }
    assert_eq!(symbol_position(&terminal, "A", 8), (0, 0));
    assert_eq!(symbol_position(&terminal, "B", 8), (0, 1));
    assert_eq!(symbol_position(&terminal, "C", 8), (0, 2));
    Ok(())
}

/// Verifies nested automatic blocks contribute their intrinsic block sizes.
///
/// # Example Under Test
///
/// ```text
/// div([
///   "A",
///   div(["B", "C"]),
///   "D",
/// ])
/// terminal size: 10x6
/// ```
///
/// # Assertions
///
/// - The nested block contributes two intrinsic rows to its parent.
/// - The root block resolves to the four-row sum of its descendants.
/// - The nested children and following sibling render on consecutive rows.
#[test]
fn nested_blocks_compose_intrinsic_block_sizes() -> Result<()> {
    let root = div((text("A"), div((text("B"), text("C"))), text("D"))).into_view();

    let terminal = render_layout_root(&root, 10, 6)?;

    let root_geometry = root
        .style_metadata()
        .and_then(StyleMetadata::layout_geometry)
        .expect("root block geometry");
    assert_eq!(
        root_geometry.border_box,
        ratatui::layout::Rect::new(0, 0, 10, 4)
    );

    let children = root
        .downcast_ref::<leptatui::DivView>()
        .expect("Div root")
        .child_views();
    let nested_geometry = children[1]
        .style_metadata()
        .and_then(StyleMetadata::layout_geometry)
        .expect("nested block geometry");
    assert_eq!(
        nested_geometry.border_box,
        ratatui::layout::Rect::new(0, 1, 10, 2)
    );
    assert_eq!(symbol_position(&terminal, "A", 10), (0, 0));
    assert_eq!(symbol_position(&terminal, "B", 10), (0, 1));
    assert_eq!(symbol_position(&terminal, "C", 10), (0, 2));
    assert_eq!(symbol_position(&terminal, "D", 10), (0, 3));
    Ok(())
}

/// Verifies hidden nodes and structural boundaries do not add block boxes.
///
/// # Example Under Test
///
/// ```text
/// div([
///   "A",
///   component("B"),
///   dynamic(|| "C"),
///   display-none "Hidden",
///   "D",
/// ])
/// ```
///
/// # Assertions
///
/// - Component and dynamic boundaries expose no styleable box of their own.
/// - Their materialized children occupy one normal-flow row each.
/// - The hidden child retains no geometry or rendered text.
/// - The final visible child follows immediately after the structural children.
#[test]
fn structural_boundaries_and_hidden_nodes_add_no_block_box() -> Result<()> {
    let hidden = text("Hidden").with_inline_style(TuiStyle::new().display(Display::None));
    let root = div((
        text("A"),
        component(text("B")),
        dynamic(|| text("C")),
        hidden,
        text("D"),
    ))
    .into_view();

    let terminal = render_layout_root(&root, 10, 5)?;

    let children = root
        .downcast_ref::<leptatui::DivView>()
        .expect("Div root")
        .child_views();
    assert!(children[1].style_metadata().is_none());
    assert!(children[2].style_metadata().is_none());
    assert!(
        children[3]
            .style_metadata()
            .and_then(StyleMetadata::layout_geometry)
            .is_none()
    );
    assert!(!rendered_text(&terminal).contains("Hidden"));
    assert_eq!(symbol_position(&terminal, "A", 10), (0, 0));
    assert_eq!(symbol_position(&terminal, "B", 10), (0, 1));
    assert_eq!(symbol_position(&terminal, "C", 10), (0, 2));
    assert_eq!(symbol_position(&terminal, "D", 10), (0, 3));
    Ok(())
}
