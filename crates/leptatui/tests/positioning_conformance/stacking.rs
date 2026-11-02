//! Positioned stacking-order and context conformance tests.

use super::*;

/// Verifies transparent boundaries preserve their positioned root's stacking.
///
/// # Example Under Test
///
/// ```text
/// relative 1x1 root
/// dynamic absolute "H" at z-index 2
/// direct absolute "L" at z-index 1
/// both at top: 0 and left: 0
/// ```
///
/// # Assertions
///
/// - The dynamic boundary contributes no independent stacking category.
/// - The dynamic positioned root paints above the lower direct sibling.
///
/// # Why
///
/// Layout-transparent boundaries must forward the paint style of their
/// materialized root instead of being classified as static normal flow.
#[test]
fn transparent_boundaries_preserve_positioned_stacking() -> leptatui::app::Result<()> {
    let inset = cell_insets(Some(0.0), None, None, Some(0.0));
    let dynamic_high = dynamic(move || {
        text("H").with_inline_style(
            fixture_size(1.0, 1.0)
                .position(Position::Absolute)
                .inset(inset)
                .z_index(ZIndex::Integer(2)),
        )
    });
    let direct_low = text("L").with_inline_style(
        fixture_size(1.0, 1.0)
            .position(Position::Absolute)
            .inset(inset)
            .z_index(ZIndex::Integer(1)),
    );
    let root = div((dynamic_high, direct_low))
        .with_inline_style(fixture_size(1.0, 1.0).position(Position::Relative))
        .into_view();

    let terminal = render_view(root.as_view(), 1, 1)?;

    assert_eq!(rendered_lines(&terminal)[0], "H");
    Ok(())
}

/// Verifies automatic z-index parents do not trap positioned descendants.
///
/// # Example Under Test
///
/// ```text
/// relative 1x1 root
/// absolute automatic-z-index parent
/// parent child "H" at z-index 100
/// root sibling "M" at z-index 1
/// all boxes overlap at top: 0 and left: 0
/// ```
///
/// # Assertions
///
/// - The automatic parent does not establish an atomic stacking context.
/// - The nested level-100 child paints above the root's level-one sibling.
///
/// # Why
///
/// Only explicit integer z-index values establish contexts; positioned
/// descendants must otherwise participate in the nearest explicit ancestor
/// context.
#[test]
fn automatic_z_index_parent_does_not_trap_positioned_descendants() -> leptatui::app::Result<()> {
    let inset = cell_insets(Some(0.0), None, None, Some(0.0));
    let high = text("H").with_inline_style(
        fixture_size(1.0, 1.0)
            .position(Position::Absolute)
            .inset(inset)
            .z_index(ZIndex::Integer(100)),
    );
    let automatic_parent = div((high,)).with_inline_style(
        fixture_size(1.0, 1.0)
            .position(Position::Absolute)
            .inset(inset),
    );
    let middle = text("M").with_inline_style(
        fixture_size(1.0, 1.0)
            .position(Position::Absolute)
            .inset(inset)
            .z_index(ZIndex::Integer(1)),
    );
    let root = div((automatic_parent, middle))
        .with_inline_style(fixture_size(1.0, 1.0).position(Position::Relative))
        .into_view();

    let terminal = render_view(root.as_view(), 1, 1)?;

    assert_eq!(rendered_lines(&terminal)[0], "H");
    Ok(())
}

/// Verifies promoted descendants preserve earlier sibling hit areas.
///
/// # Example Under Test
///
/// ```text
/// relative 8x3 root
/// dynamic absolute automatic-z-index parent
/// static 5x3 button inside the parent
/// absolute descendant at left: 6 and z-index 10
/// MouseMoved(2, 1)
/// ```
///
/// # Assertions
///
/// - Replaying the promoted descendant through the dynamic boundary does not
///   clear the static button's earlier hit area.
/// - Pointer movement over the static button focuses it.
///
/// # Why
///
/// Transparent-boundary traversal renders an automatic-context parent once
/// for its shell and static children, then re-enters it for each promoted
/// positioned descendant. Re-entry must retain interaction geometry recorded
/// by the shell pass.
#[test]
fn promoted_descendant_replay_preserves_static_sibling_hit_area() -> leptatui::app::Result<()> {
    let parent_inset = cell_insets(Some(0.0), None, None, Some(0.0));
    let descendant_inset = cell_insets(Some(0.0), None, None, Some(6.0));
    let dynamic_parent = dynamic(move || {
        let static_button = button("Stay").with_inline_style(fixture_size(5.0, 3.0));
        let promoted = text("H").with_inline_style(
            fixture_size(1.0, 1.0)
                .position(Position::Absolute)
                .inset(descendant_inset)
                .z_index(ZIndex::Integer(10)),
        );
        div((static_button, promoted)).with_inline_style(
            fixture_size(8.0, 3.0)
                .position(Position::Absolute)
                .inset(parent_inset),
        )
    });
    let mut root = div((dynamic_parent,))
        .with_inline_style(fixture_size(8.0, 3.0).position(Position::Relative))
        .into_view();

    let _terminal = render_view(root.as_view(), 8, 3)?;
    root.handle_event(mouse(MouseEventKind::Moved, 2, 1))?;

    assert_eq!(root.__focused_control(), Some(FocusedControl::Button));
    Ok(())
}

/// Verifies positioned layers surround normal flow in deterministic order.
///
/// # Example Under Test
///
/// ```text
/// four 1x1 overlapping roots:
/// negative positioned after normal flow
/// automatic positioned before static z-index 100
/// explicit zero before automatic positioned
/// positive z-index 2 before positive z-index 1
/// ```
///
/// # Assertions
///
/// - Normal flow paints over a later negative positioned box.
/// - Automatic positioned content paints over a later static box whose
///   authored z-index is ignored.
/// - Automatic and explicit zero levels use source order.
/// - Larger positive levels paint over smaller levels regardless of source
///   order.
#[test]
fn positioned_layers_surround_normal_flow_and_break_ties_by_source_order()
-> leptatui::app::Result<()> {
    let inset = cell_insets(Some(0.0), None, None, Some(0.0));
    let root_style = fixture_size(1.0, 1.0)
        .position(Position::Relative)
        .overflow(Axes::all(Overflow::Visible));
    let positioned = |label, z_index| {
        text(label).with_inline_style(
            fixture_size(1.0, 1.0)
                .position(Position::Absolute)
                .inset(inset)
                .z_index(z_index),
        )
    };

    let negative = div((text("N"), positioned("X", ZIndex::Integer(-1))))
        .with_inline_style(root_style.clone())
        .into_view();
    assert_eq!(
        rendered_lines(&render_view(negative.as_view(), 1, 1)?)[0],
        "N"
    );

    let automatic = text("A").with_inline_style(
        fixture_size(1.0, 1.0)
            .position(Position::Absolute)
            .inset(inset),
    );
    let static_with_z_index =
        text("S").with_inline_style(TuiStyle::new().z_index(ZIndex::Integer(100)));
    let normal_then_positioned = div((automatic, static_with_z_index))
        .with_inline_style(root_style.clone())
        .into_view();
    assert_eq!(
        rendered_lines(&render_view(normal_then_positioned.as_view(), 1, 1)?)[0],
        "A"
    );

    let zero_then_auto = div((
        positioned("Z", ZIndex::Integer(0)),
        text("A").with_inline_style(
            fixture_size(1.0, 1.0)
                .position(Position::Absolute)
                .inset(inset),
        ),
    ))
    .with_inline_style(root_style.clone())
    .into_view();
    assert_eq!(
        rendered_lines(&render_view(zero_then_auto.as_view(), 1, 1)?)[0],
        "A"
    );

    let positive = div((
        positioned("H", ZIndex::Integer(2)),
        positioned("L", ZIndex::Integer(1)),
    ))
    .with_inline_style(root_style)
    .into_view();
    assert_eq!(
        rendered_lines(&render_view(positive.as_view(), 1, 1)?)[0],
        "H"
    );
    Ok(())
}

/// Verifies normal-flow overlaps retain source-order painting.
///
/// # Example Under Test
///
/// ```text
/// 1x1 grid
/// static "A" in grid area 1 / 1
/// static "B" in the same grid area
/// ```
///
/// # Assertions
///
/// - Both static children occupy the same retained grid cell.
/// - The later static child paints over the earlier child.
#[test]
fn overlapping_normal_flow_grid_items_paint_in_source_order() -> leptatui::app::Result<()> {
    let placement = TuiStyle::new()
        .grid_row(GridLine::new(
            GridPlacement::line(1),
            GridPlacement::line(2),
        ))
        .grid_column(GridLine::new(
            GridPlacement::line(1),
            GridPlacement::line(2),
        ));
    let root = div((
        text("A").with_inline_style(placement.clone()),
        text("B").with_inline_style(placement),
    ))
    .with_inline_style(
        fixture_size(1.0, 1.0)
            .display(Display::Grid)
            .grid_template_columns(vec![GridTemplateTrack::from(GridTrackSize::from(
                Length::cells(1.0),
            ))])
            .grid_template_rows(vec![GridTemplateTrack::from(GridTrackSize::from(
                Length::cells(1.0),
            ))]),
    )
    .into_view();

    let terminal = render_view(root.as_view(), 1, 1)?;
    let rects = retained_child_rects(&root);

    assert_eq!(rects, vec![Rect::new(0, 0, 1, 1); 2]);
    assert_eq!(rendered_lines(&terminal)[0], "B");
    Ok(())
}

/// Verifies explicit nested contexts keep descendant levels locally isolated.
///
/// # Example Under Test
///
/// ```text
/// root 1: parent z-index -1 containing child z-index 100, sibling z-index 0
/// root 2: sibling z-index 0, parent z-index 1 containing child z-index -100
/// ```
///
/// # Assertions
///
/// - A high descendant cannot escape its negative parent context.
/// - A negative descendant remains above a sibling behind its positive parent
///   context.
#[test]
fn explicit_nested_stacking_contexts_are_atomic_between_siblings() -> leptatui::app::Result<()> {
    let inset = cell_insets(Some(0.0), None, None, Some(0.0));
    let root_style = fixture_size(1.0, 1.0).position(Position::Relative);
    let context = |label, parent_level, child_level| {
        div((text(label).with_inline_style(
            fixture_size(1.0, 1.0)
                .position(Position::Absolute)
                .inset(inset)
                .z_index(ZIndex::Integer(child_level)),
        ),))
        .with_inline_style(
            fixture_size(1.0, 1.0)
                .position(Position::Absolute)
                .inset(inset)
                .z_index(ZIndex::Integer(parent_level)),
        )
    };
    let sibling = || {
        text("M").with_inline_style(
            fixture_size(1.0, 1.0)
                .position(Position::Absolute)
                .inset(inset)
                .z_index(ZIndex::Integer(0)),
        )
    };

    let trapped_high = div((context("H", -1, 100), sibling()))
        .with_inline_style(root_style.clone())
        .into_view();
    assert_eq!(
        rendered_lines(&render_view(trapped_high.as_view(), 1, 1)?)[0],
        "M"
    );

    let retained_low = div((sibling(), context("L", 1, -100)))
        .with_inline_style(root_style)
        .into_view();
    assert_eq!(
        rendered_lines(&render_view(retained_low.as_view(), 1, 1)?)[0],
        "L"
    );
    Ok(())
}

/// Verifies context chrome precedes negative children and clipping still applies.
///
/// # Example Under Test
///
/// ```text
/// 3x3 blue relative box with borders
/// 4x1 red absolute child at z-index -1
/// hidden overflow
/// terminal size: 4x3
/// ```
///
/// # Assertions
///
/// - The parent border remains painted around its content box.
/// - The negative child paints over the parent's content background.
/// - The child remains clipped at the parent's right border.
#[test]
fn negative_children_paint_over_context_background_without_escaping_clip()
-> leptatui::app::Result<()> {
    let child = div((text("XXXX"),)).with_inline_style(
        fixture_size(4.0, 1.0)
            .background(Color::Red)
            .position(Position::Absolute)
            .inset(cell_insets(Some(0.0), None, None, Some(0.0)))
            .z_index(ZIndex::Integer(-1)),
    );
    let root = div((child,))
        .with_inline_style(
            fixture_size(3.0, 3.0)
                .background(Color::Blue)
                .borders(Borders::ALL)
                .position(Position::Relative)
                .overflow(Axes::all(Overflow::Hidden)),
        )
        .into_view();

    let terminal = render_view(root.as_view(), 4, 3)?;
    let lines = rendered_lines(&terminal);

    assert_eq!(lines[0], "┌─┐ ");
    assert_eq!(lines[1], "│X│ ");
    assert_eq!(terminal.backend().buffer()[(1, 1)].bg, Color::Red,);
    assert_eq!(terminal.backend().buffer()[(3, 1)].bg, Color::Reset);
    Ok(())
}

/// Verifies fixed boxes use viewport stacking levels instead of traversal order.
///
/// # Example Under Test
///
/// ```text
/// fixed "H" at z-index 2
/// fixed "L" at z-index 1 later in source order
/// both at viewport row 0, column 0
/// ```
///
/// # Assertions
///
/// - The earlier higher-level fixed box paints over the later lower-level box.
#[test]
fn fixed_descendants_paint_in_viewport_stacking_order() -> leptatui::app::Result<()> {
    let inset = cell_insets(Some(0.0), None, None, Some(0.0));
    let fixed = |label, level| {
        text(label).with_inline_style(
            fixture_size(1.0, 1.0)
                .position(Position::Fixed)
                .inset(inset)
                .z_index(ZIndex::Integer(level)),
        )
    };
    let root = div((fixed("H", 2), fixed("L", 1)))
        .with_inline_style(fixture_size(1.0, 1.0))
        .into_view();

    let terminal = render_view(root.as_view(), 1, 1)?;

    assert_eq!(rendered_lines(&terminal)[0], "H");
    Ok(())
}

/// Verifies pointer targeting follows the final global fixed paint pass.
///
/// # Example Under Test
///
/// ```text
/// relative 6x3 root
/// first branch contains a fixed button at viewport origin
/// later branch contains an absolute button at the same origin
/// MouseMoved(2, 1)
/// ```
///
/// # Assertions
///
/// - Pointer movement focuses the fixed button.
/// - Logical button focus state remains source-ordered.
///
/// # Why
///
/// Direct-child ordering cannot represent fixed descendants that escape an
/// earlier logical branch and paint during the deferred viewport pass.
#[test]
fn pointer_targeting_uses_global_fixed_paint_order() -> leptatui::app::Result<()> {
    let inset = cell_insets(Some(0.0), None, None, Some(0.0));
    let fixed = button("Fixed").with_inline_style(
        fixture_size(6.0, 3.0)
            .position(Position::Fixed)
            .inset(inset),
    );
    let absolute = button("Under").with_inline_style(
        fixture_size(6.0, 3.0)
            .position(Position::Absolute)
            .inset(inset)
            .z_index(ZIndex::Integer(10)),
    );
    let mut root = div((div((fixed,)), absolute))
        .with_inline_style(fixture_size(6.0, 3.0).position(Position::Relative))
        .into_view();

    let _terminal = render_view(root.as_view(), 6, 3)?;

    root.handle_event(mouse(MouseEventKind::Moved, 2, 1))?;

    assert_eq!(button_focuses(root.as_view()), vec![true, false]);
    Ok(())
}
