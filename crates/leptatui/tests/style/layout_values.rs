/// Typed layout value construction, defaults, and public export tests.
mod layout_values {
    use leptatui::{
        AlignContent, AlignItems, AlignSelf, Axes, BoxSizing, Dimension, Display, Edges,
        FlexDirection, FlexWrap, Fraction, GridAutoFlow, GridLine, GridMaxTrackSize,
        GridMinTrackSize, GridPlacement, GridRepeat, GridTemplateTrack, GridTrackSize,
        JustifyContent, JustifyItems, JustifySelf, LayoutSize, Length, LengthAuto, Overflow,
        Position, ZIndex,
    };

    /// Verifies every definite length unit and intrinsic dimension can be constructed.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// Length::cells(4.0)
    /// Length::percent(50.0)
    /// Length::vw(25.0)
    /// Dimension::MinContent
    /// Dimension::FitContent(Length::cells(12.0))
    /// ```
    ///
    /// # Assertions
    ///
    /// - Every length constructor selects its corresponding public variant.
    /// - A definite length converts into `LengthAuto::Length`.
    /// - A definite length converts into `Dimension::Length`.
    /// - Automatic and intrinsic dimension variants retain their authored values.
    /// - A floating-point value converts into a matching grid fraction.
    #[test]
    fn layout_lengths_and_dimensions_retain_authored_values() {
        let lengths = [
            (Length::cells(4.0), Length::Cells(4.0)),
            (Length::percent(50.0), Length::Percent(50.0)),
            (Length::vw(25.0), Length::ViewportWidth(25.0)),
            (Length::vh(30.0), Length::ViewportHeight(30.0)),
            (Length::vmin(40.0), Length::ViewportMin(40.0)),
            (Length::vmax(60.0), Length::ViewportMax(60.0)),
        ];

        for (actual, expected) in lengths {
            assert_eq!(actual, expected);
        }

        assert_eq!(
            LengthAuto::from(Length::cells(2.0)),
            LengthAuto::Length(Length::Cells(2.0))
        );
        assert_eq!(
            Dimension::from(Length::percent(75.0)),
            Dimension::Length(Length::Percent(75.0))
        );
        assert_eq!(Dimension::default(), Dimension::Auto);
        assert_eq!(Dimension::MinContent, Dimension::MinContent);
        assert_eq!(Dimension::MaxContent, Dimension::MaxContent);
        assert_eq!(
            Dimension::FitContent(Length::cells(12.0)),
            Dimension::FitContent(Length::Cells(12.0))
        );
        assert_eq!(Fraction::from(2.5), Fraction::new(2.5));
    }

    /// Verifies public grid sizing types represent every supported track form.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// minmax(2 cells, 3fr)
    /// repeat(2, 25% 1fr)
    /// repeat(auto-fill, 25% 1fr)
    /// repeat(auto-fit, 25% 1fr)
    /// ```
    ///
    /// # Assertions
    ///
    /// - Fixed, percentage, fractional, automatic, and intrinsic tracks retain their values.
    /// - `minmax()` keeps independently typed minimum and maximum bounds.
    /// - Counted, auto-fill, and auto-fit repetitions retain their fragments.
    #[test]
    fn grid_track_types_retain_authored_templates() {
        let minmax = GridTrackSize::minmax(
            GridMinTrackSize::Length(Length::cells(2.0)),
            GridMaxTrackSize::Fraction(Fraction::new(3.0)),
        );
        let fragment = vec![
            GridTrackSize::from(Length::percent(25.0)),
            GridTrackSize::from(Fraction::new(1.0)),
        ];
        let templates = vec![
            GridTemplateTrack::from(GridTrackSize::Length(Length::cells(4.0))),
            GridTemplateTrack::repeat(GridRepeat::count(2), fragment.clone()),
            GridTemplateTrack::repeat(GridRepeat::AutoFill, fragment.clone()),
            GridTemplateTrack::repeat(GridRepeat::AutoFit, fragment.clone()),
        ];

        assert_eq!(
            minmax,
            GridTrackSize::MinMax {
                min: GridMinTrackSize::Length(Length::Cells(2.0)),
                max: GridMaxTrackSize::Fraction(Fraction::new(3.0)),
            }
        );
        assert_eq!(GridTrackSize::Auto, GridTrackSize::Auto);
        assert_eq!(GridTrackSize::MinContent, GridTrackSize::MinContent);
        assert_eq!(GridTrackSize::MaxContent, GridTrackSize::MaxContent);
        assert_eq!(
            templates[0],
            GridTemplateTrack::Track(GridTrackSize::Length(Length::Cells(4.0)))
        );
        assert_eq!(
            templates[1],
            GridTemplateTrack::Repeat {
                repetition: GridRepeat::Count(2),
                tracks: fragment.clone(),
            }
        );
        assert_eq!(
            templates[2],
            GridTemplateTrack::Repeat {
                repetition: GridRepeat::AutoFill,
                tracks: fragment.clone(),
            }
        );
        assert_eq!(
            templates[3],
            GridTemplateTrack::Repeat {
                repetition: GridRepeat::AutoFit,
                tracks: fragment,
            }
        );
    }

    /// Verifies generic geometry helpers preserve physical and axis ordering.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// Edges::new(1, 2, 3, 4)
    /// Edges::symmetric(2, 1)
    /// Axes::new(3, 4)
    /// LayoutSize::new(5, 6)
    /// ```
    ///
    /// # Assertions
    ///
    /// - Independent edge values remain in top, right, bottom, and left order.
    /// - Uniform and symmetric edge helpers apply values to the expected edges.
    /// - Axis and size helpers preserve independent values and duplicate uniform values.
    /// - Generic geometry defaults every component through the contained type.
    #[test]
    fn generic_layout_geometry_preserves_component_order() {
        assert_eq!(
            Edges::new(1, 2, 3, 4),
            Edges {
                top: 1,
                right: 2,
                bottom: 3,
                left: 4,
            }
        );
        assert_eq!(Edges::all(3), Edges::new(3, 3, 3, 3));
        assert_eq!(Edges::symmetric(2, 1), Edges::new(1, 2, 1, 2));
        assert_eq!(Axes::new(3, 4), Axes { x: 3, y: 4 });
        assert_eq!(Axes::all(5), Axes::new(5, 5));
        assert_eq!(
            LayoutSize::new(6, 7),
            LayoutSize {
                width: 6,
                height: 7,
            }
        );
        assert_eq!(LayoutSize::all(8), LayoutSize::new(8, 8));
        assert_eq!(Edges::<u16>::default(), Edges::all(0));
        assert_eq!(Axes::<u16>::default(), Axes::all(0));
        assert_eq!(LayoutSize::<u16>::default(), LayoutSize::all(0));
    }

    /// Verifies layout enums expose stable CSS-oriented initial values.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// Display::default()
    /// BoxSizing::default()
    /// Overflow::default()
    /// FlexDirection::default()
    /// ```
    ///
    /// # Assertions
    ///
    /// - Boxes default to block display and content-box sizing.
    /// - Overflow defaults to visible on each independently representable axis.
    /// - Flexbox defaults to row direction without wrapping.
    /// - Alignment defaults match automatic or stretching item behavior and start packing.
    /// - Grid, positioning, placement, and stacking values use their automatic initial states.
    #[test]
    fn layout_value_defaults_match_public_initial_states() {
        assert_eq!(Display::default(), Display::Block);
        assert_eq!(BoxSizing::default(), BoxSizing::ContentBox);
        assert_eq!(Overflow::default(), Overflow::Visible);
        assert_eq!(FlexDirection::default(), FlexDirection::Row);
        assert_eq!(FlexWrap::default(), FlexWrap::NoWrap);
        assert_eq!(AlignItems::default(), AlignItems::Stretch);
        assert_eq!(AlignSelf::default(), AlignSelf::Auto);
        assert_eq!(AlignContent::default(), AlignContent::Stretch);
        assert_eq!(JustifyItems::default(), JustifyItems::Stretch);
        assert_eq!(JustifySelf::default(), JustifySelf::Auto);
        assert_eq!(JustifyContent::default(), JustifyContent::Start);
        assert_eq!(GridAutoFlow::default(), GridAutoFlow::Row);
        assert_eq!(GridPlacement::default(), GridPlacement::Auto);
        assert_eq!(GridLine::default(), GridLine::new(GridPlacement::Auto, GridPlacement::Auto));
        assert_eq!(Position::default(), Position::Static);
        assert_eq!(ZIndex::default(), ZIndex::Auto);
    }

    /// Verifies flexbox, grid, overflow, positioning, and stacking variants are constructible.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// GridLine::new(GridPlacement::line(1), GridPlacement::span(2))
    /// Axes::new(Overflow::Hidden, Overflow::Auto)
    /// ZIndex::from(-1)
    /// ```
    ///
    /// # Assertions
    ///
    /// - Every display, overflow, flex direction, flex wrap, and grid-flow variant is public.
    /// - Every alignment variant is public.
    /// - Signed grid lines, spans, and start/end pairs preserve their values.
    /// - Every positioning mode and explicit signed z-index is public.
    #[test]
    fn public_layout_variants_are_constructible() {
        let displays = [Display::Block, Display::Flex, Display::Grid, Display::None];
        let overflows = [
            Overflow::Visible,
            Overflow::Hidden,
            Overflow::Clip,
            Overflow::Scroll,
            Overflow::Auto,
        ];
        let directions = [
            FlexDirection::Row,
            FlexDirection::RowReverse,
            FlexDirection::Column,
            FlexDirection::ColumnReverse,
        ];
        let wraps = [FlexWrap::NoWrap, FlexWrap::Wrap, FlexWrap::WrapReverse];
        let flows = [
            GridAutoFlow::Row,
            GridAutoFlow::Column,
            GridAutoFlow::RowDense,
            GridAutoFlow::ColumnDense,
        ];
        let align_items = [
            AlignItems::Start,
            AlignItems::End,
            AlignItems::FlexStart,
            AlignItems::FlexEnd,
            AlignItems::Center,
            AlignItems::Baseline,
            AlignItems::Stretch,
        ];
        let align_self = [
            AlignSelf::Auto,
            AlignSelf::Start,
            AlignSelf::End,
            AlignSelf::FlexStart,
            AlignSelf::FlexEnd,
            AlignSelf::Center,
            AlignSelf::Baseline,
            AlignSelf::Stretch,
        ];
        let align_content = [
            AlignContent::Start,
            AlignContent::End,
            AlignContent::FlexStart,
            AlignContent::FlexEnd,
            AlignContent::Center,
            AlignContent::Stretch,
            AlignContent::SpaceBetween,
            AlignContent::SpaceAround,
            AlignContent::SpaceEvenly,
        ];
        let justify_items = [
            JustifyItems::Start,
            JustifyItems::End,
            JustifyItems::Center,
            JustifyItems::Baseline,
            JustifyItems::Stretch,
        ];
        let justify_self = [
            JustifySelf::Auto,
            JustifySelf::Start,
            JustifySelf::End,
            JustifySelf::Center,
            JustifySelf::Baseline,
            JustifySelf::Stretch,
        ];
        let justify_content = [
            JustifyContent::Start,
            JustifyContent::End,
            JustifyContent::FlexStart,
            JustifyContent::FlexEnd,
            JustifyContent::Center,
            JustifyContent::Stretch,
            JustifyContent::SpaceBetween,
            JustifyContent::SpaceAround,
            JustifyContent::SpaceEvenly,
        ];
        let positions = [
            Position::Static,
            Position::Relative,
            Position::Absolute,
            Position::Fixed,
            Position::Sticky,
        ];

        assert_eq!(displays.len(), 4);
        assert_eq!(overflows.len(), 5);
        assert_eq!(directions.len(), 4);
        assert_eq!(wraps.len(), 3);
        assert_eq!(flows.len(), 4);
        assert_eq!(align_items.len(), 7);
        assert_eq!(align_self.len(), 8);
        assert_eq!(align_content.len(), 9);
        assert_eq!(justify_items.len(), 5);
        assert_eq!(justify_self.len(), 6);
        assert_eq!(justify_content.len(), 9);
        assert_eq!(positions.len(), 5);

        assert_eq!(GridPlacement::line(-2), GridPlacement::Line(-2));
        assert_eq!(GridPlacement::span(3), GridPlacement::Span(3));
        assert_eq!(
            GridLine::new(GridPlacement::line(1), GridPlacement::span(2)),
            GridLine {
                start: GridPlacement::Line(1),
                end: GridPlacement::Span(2),
            }
        );
        assert_eq!(
            Axes::new(Overflow::Hidden, Overflow::Auto),
            Axes {
                x: Overflow::Hidden,
                y: Overflow::Auto,
            }
        );
        assert_eq!(ZIndex::from(-1), ZIndex::Integer(-1));
    }

    /// Verifies the default prelude exports the complete layout value vocabulary.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// use leptatui::prelude::*;
    /// LayoutSize::new(Dimension::Auto, Dimension::MinContent)
    /// ```
    ///
    /// # Assertions
    ///
    /// - Prelude imports expose geometry, display, flexbox, grid, positioning, and stacking types.
    /// - The imported types compose without a layout-engine dependency.
    #[test]
    fn prelude_exports_layout_values_without_engine_types() {
        use leptatui::prelude::*;

        let size = LayoutSize::new(Dimension::Auto, Dimension::MinContent);
        let flow = (Display::Grid, GridAutoFlow::ColumnDense);
        let placement = GridLine::new(GridPlacement::line(1), GridPlacement::span(2));
        let position = (
            Position::Absolute,
            Edges::all(LengthAuto::from(Length::cells(1.0))),
            ZIndex::Integer(3),
        );

        assert_eq!(size.height, Dimension::MinContent);
        assert_eq!(flow, (Display::Grid, GridAutoFlow::ColumnDense));
        assert_eq!(placement.end, GridPlacement::Span(2));
        assert_eq!(position.2, ZIndex::Integer(3));
    }
}
