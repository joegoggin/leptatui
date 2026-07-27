//! Pass fixture for typed layout declarations and imported variables.

use leptatui::prelude::*;

/// Defines reusable layout values for imported stylesheet declarations.
fn layout_variables() -> StyleModule {
    stylesheet! {
        $display: Display::Grid;
        $box_sizing: BoxSizing::BorderBox;
        $overflow: Axes::new(Overflow::Hidden, Overflow::Auto);
        $size: LayoutSize::new(Dimension::MinContent, Dimension::MaxContent);
        $min_size: LayoutSize::all(Dimension::from(Length::cells(2.0)));
        $max_size: LayoutSize::all(Dimension::FitContent(Length::cells(40.0)));
        $aspect_ratio: 1.5_f32;
        $margin: Edges::all(LengthAuto::from(Length::cells(1.0)));
        $gap: Axes::new(Length::cells(2.0), Length::cells(3.0));
        $flex_direction: FlexDirection::ColumnReverse;
        $flex_wrap: FlexWrap::WrapReverse;
        $flex_basis: Dimension::from(Length::percent(25.0));
        $flex_grow: 2.0_f32;
        $flex_shrink: 0.5_f32;
        $align_items: AlignItems::Center;
        $align_self: AlignSelf::FlexEnd;
        $align_content: AlignContent::SpaceAround;
        $justify_items: JustifyItems::End;
        $justify_self: JustifySelf::Center;
        $justify_content: JustifyContent::SpaceEvenly;
        $grid_auto_flow: GridAutoFlow::ColumnDense;
        $grid_template: vec![
            GridTemplateTrack::from(GridTrackSize::from(Length::cells(4.0))),
            GridTemplateTrack::repeat(
                GridRepeat::count(2),
                vec![GridTrackSize::from(Fraction::new(1.0))],
            ),
        ];
        $grid_auto_tracks: vec![
            GridTrackSize::minmax(
                GridMinTrackSize::MinContent,
                GridMaxTrackSize::MaxContent,
            ),
        ];
        $grid_line: GridLine::new(GridPlacement::line(2), GridPlacement::span(3));
        $position: Position::Absolute;
        $inset: Edges::symmetric(LengthAuto::Auto, Length::cells(4.0).into());
        $z_index: ZIndex::Integer(7);
    }
}

/// Exercises every layout declaration through imported typed variables.
fn main() {
    let styles = stylesheet! {
        @use layout_variables as layout;

        .panel => {
            display: layout.$display !important,
            box_sizing: layout.$box_sizing,
            overflow: layout.$overflow,
            size: layout.$size,
            min_size: layout.$min_size,
            max_size: layout.$max_size,
            aspect_ratio: layout.$aspect_ratio,
            margin: layout.$margin,
            gap: layout.$gap,
            flex_direction: layout.$flex_direction,
            flex_wrap: layout.$flex_wrap,
            flex_basis: layout.$flex_basis,
            flex_grow: layout.$flex_grow,
            flex_shrink: layout.$flex_shrink,
            align_items: layout.$align_items,
            align_self: layout.$align_self,
            align_content: layout.$align_content,
            justify_items: layout.$justify_items,
            justify_self: layout.$justify_self,
            justify_content: layout.$justify_content,
            grid_auto_flow: layout.$grid_auto_flow,
            grid_template_rows: layout.$grid_template,
            grid_template_columns: layout.$grid_template,
            grid_auto_rows: layout.$grid_auto_tracks,
            grid_auto_columns: layout.$grid_auto_tracks,
            grid_row: layout.$grid_line,
            grid_column: layout.$grid_line,
            position: layout.$position,
            inset: layout.$inset,
            z_index: layout.$z_index
        }
    };
    let view = text("Panel").with_classes("panel");
    let resolved = styles.resolve(
        view.style_metadata().unwrap(),
        &[],
        TuiStyle::new(),
        &ThemeVariables::new(),
    );

    assert_eq!(resolved.display, Some(Display::Grid));
    assert_eq!(resolved.box_sizing, Some(BoxSizing::BorderBox));
    assert_eq!(
        resolved.overflow,
        Some(Axes::new(Overflow::Hidden, Overflow::Auto))
    );
    assert_eq!(resolved.aspect_ratio, Some(1.5));
    assert_eq!(resolved.grid_template_rows.as_ref().map(Vec::len), Some(2));
    assert_eq!(
        resolved.grid_template_columns.as_ref().map(Vec::len),
        Some(2)
    );
    assert_eq!(resolved.grid_auto_rows.as_ref().map(Vec::len), Some(1));
    assert_eq!(resolved.grid_auto_columns.as_ref().map(Vec::len), Some(1));
    assert_eq!(resolved.position, Some(Position::Absolute));
    assert_eq!(resolved.z_index, Some(ZIndex::Integer(7)));
}
