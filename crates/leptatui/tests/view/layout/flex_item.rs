/// Styleable leaf that records intrinsic measurements requested by flex layout.
struct FlexMeasurementProbe {
    /// Selector and retained layout metadata.
    metadata: StyleMetadata,
    /// Intrinsic terminal-cell dimensions returned before flex sizing.
    intrinsic_size: LayoutSize<f32>,
    /// Number of measurement requests received from the layout engine.
    measure_count: Rc<Cell<usize>>,
}

impl View for FlexMeasurementProbe {
    /// Paints no content after layout assigns the probe geometry.
    ///
    /// # Arguments
    ///
    /// * `_ctx` — Rendering context containing the assigned geometry.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] because the probe performs no terminal I/O.
    fn render(&self, _ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        Ok(())
    }

    /// Records the request and returns intrinsic or engine-constrained geometry.
    ///
    /// # Arguments
    ///
    /// * `known_dimensions` — Exact dimensions assigned by flex layout.
    /// * `_available_space` — Soft constraints supplied by flex layout.
    /// * `_ctx` — Rendering context active during measurement.
    ///
    /// # Returns
    ///
    /// A [`LayoutSize`] containing the constrained or intrinsic size.
    fn measure(
        &self,
        known_dimensions: LayoutSize<Option<f32>>,
        _available_space: LayoutSize<AvailableSpace>,
        _ctx: &mut RenderCtx<'_, '_>,
    ) -> LayoutSize<f32> {
        self.measure_count
            .set(self.measure_count.get().saturating_add(1));
        LayoutSize::new(
            known_dimensions.width.unwrap_or(self.intrinsic_size.width),
            known_dimensions.height.unwrap_or(self.intrinsic_size.height),
        )
    }

    /// Returns the probe's selector and retained layout metadata.
    ///
    /// # Returns
    ///
    /// An optional shared [`StyleMetadata`] reference.
    fn style_metadata(&self) -> Option<&StyleMetadata> {
        Some(&self.metadata)
    }

    /// Returns mutable selector and retained layout metadata.
    ///
    /// # Returns
    ///
    /// An optional mutable [`StyleMetadata`] reference.
    fn style_metadata_mut(&mut self) -> Option<&mut StyleMetadata> {
        Some(&mut self.metadata)
    }

    /// Returns the probe for shared concrete-type inspection.
    ///
    /// # Returns
    ///
    /// A shared [`std::any::Any`] trait object.
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    /// Returns the probe for mutable concrete-type inspection.
    ///
    /// # Returns
    ///
    /// A mutable [`std::any::Any`] trait object.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Creates one styleable intrinsic-measurement probe.
///
/// # Arguments
///
/// * `intrinsic_size` — Natural size returned before flex sizing.
/// * `style` — Flex-item style applied to the probe.
///
/// # Returns
///
/// A tuple containing the erased probe and its shared measurement counter.
fn flex_measurement_probe(
    intrinsic_size: LayoutSize<f32>,
    style: TuiStyle,
) -> (AnyView, Rc<Cell<usize>>) {
    let measure_count = Rc::new(Cell::new(0));
    let mut metadata = StyleMetadata::new(ViewType::new("FlexMeasurementProbe"));
    metadata.set_inline_style(style);
    (
        FlexMeasurementProbe {
            metadata,
            intrinsic_size,
            measure_count: Rc::clone(&measure_count),
        }
        .into_view(),
        measure_count,
    )
}

/// Returns a width-only flex-item constraint.
///
/// # Arguments
///
/// * `width` — Definite main-axis constraint in terminal cells.
///
/// # Returns
///
/// A [`LayoutSize`] containing the definite width and automatic height.
fn flex_width(width: f32) -> LayoutSize<Dimension> {
    LayoutSize::new(Dimension::from(Length::cells(width)), Dimension::Auto)
}

/// Verifies authored and intrinsic flex bases determine initial item widths.
///
/// # Example Under Test
///
/// ```text
/// 12x3 flex row
/// intrinsic text: "abc"
/// authored basis: 5 cells on text "x"
/// ```
///
/// # Assertions
///
/// - The automatic basis uses the first child's three-cell intrinsic width.
/// - The authored basis overrides the second child's one-cell intrinsic width.
/// - Both items retain their intrinsic one-cell cross size.
#[test]
fn flex_basis_uses_authored_or_intrinsic_main_size() -> Result<()> {
    let root = div((
        text("abc").with_inline_style(TuiStyle::new().flex_shrink(0.0)),
        text("x").with_inline_style(
            TuiStyle::new()
                .flex_basis(Dimension::from(Length::cells(5.0)))
                .flex_shrink(0.0),
        ),
    ))
    .with_inline_style(
        flex_fixture_size(12.0, 3.0)
            .display(Display::Flex)
            .align_items(AlignItems::FlexStart),
    )
    .into_view();

    let _terminal = render_layout_root(&root, 12, 3)?;

    assert_eq!(
        retained_flex_children(&root),
        [
            ratatui::layout::Rect::new(0, 0, 3, 1),
            ratatui::layout::Rect::new(3, 0, 5, 1),
        ]
    );
    Ok(())
}

/// Verifies flex-grow factors distribute positive free space proportionally.
///
/// # Example Under Test
///
/// ```text
/// 10x2 flex row
/// bases: 2 and 2 cells
/// grow factors: 1 and 2
/// ```
///
/// # Assertions
///
/// - The first item receives two of the six free cells.
/// - The second item receives four of the six free cells.
/// - The grown items exactly cover the parent main axis.
#[test]
fn flex_grow_distributes_positive_free_space_by_factor() -> Result<()> {
    let root = div((
        text("A").with_inline_style(
            TuiStyle::new()
                .flex_basis(Dimension::from(Length::cells(2.0)))
                .flex_grow(1.0),
        ),
        text("B").with_inline_style(
            TuiStyle::new()
                .flex_basis(Dimension::from(Length::cells(2.0)))
                .flex_grow(2.0),
        ),
    ))
    .with_inline_style(
        flex_fixture_size(10.0, 2.0)
            .display(Display::Flex)
            .align_items(AlignItems::FlexStart),
    )
    .into_view();

    let _terminal = render_layout_root(&root, 10, 2)?;

    assert_eq!(
        retained_flex_children(&root),
        [
            ratatui::layout::Rect::new(0, 0, 4, 1),
            ratatui::layout::Rect::new(4, 0, 6, 1),
        ]
    );
    Ok(())
}

/// Verifies flex-shrink factors distribute negative free space.
///
/// # Example Under Test
///
/// ```text
/// 6x2 flex row
/// bases: 4 and 4 cells
/// shrink factors: 1 and 1
/// ```
///
/// # Assertions
///
/// - Each item gives up one cell from its four-cell basis.
/// - The shrunken items remain contiguous.
/// - The shrunken items exactly cover the parent main axis.
#[test]
fn flex_shrink_distributes_negative_free_space() -> Result<()> {
    let child_style = TuiStyle::new()
        .flex_basis(Dimension::from(Length::cells(4.0)))
        .flex_shrink(1.0);
    let root = div((
        text("A").with_inline_style(child_style.clone()),
        text("B").with_inline_style(child_style),
    ))
    .with_inline_style(
        flex_fixture_size(6.0, 2.0)
            .display(Display::Flex)
            .align_items(AlignItems::FlexStart),
    )
    .into_view();

    let _terminal = render_layout_root(&root, 6, 2)?;

    assert_eq!(
        retained_flex_children(&root),
        [
            ratatui::layout::Rect::new(0, 0, 3, 1),
            ratatui::layout::Rect::new(3, 0, 3, 1),
        ]
    );
    Ok(())
}

/// Verifies flex distribution freezes items at minimum and maximum widths.
///
/// # Example Under Test
///
/// ```text
/// grow: 12-cell row, 2-cell bases, first max-width 4
/// shrink: 4-cell row, 4-cell bases, first min-width 3
/// ```
///
/// # Assertions
///
/// - Growth stops the first item at four cells and gives the remainder to its sibling.
/// - Shrinkage stops the first item at three cells and reduces its sibling to one cell.
/// - Both constrained layouts exactly cover their parent main axes.
#[test]
fn flex_distribution_honors_minimum_and_maximum_widths() -> Result<()> {
    let growing = div((
        text("A").with_inline_style(
            TuiStyle::new()
                .flex_basis(Dimension::from(Length::cells(2.0)))
                .flex_grow(1.0)
                .max_size(flex_width(4.0)),
        ),
        text("B").with_inline_style(
            TuiStyle::new()
                .flex_basis(Dimension::from(Length::cells(2.0)))
                .flex_grow(1.0),
        ),
    ))
    .with_inline_style(
        flex_fixture_size(12.0, 2.0)
            .display(Display::Flex)
            .align_items(AlignItems::FlexStart),
    )
    .into_view();
    let shrinking = div((
        text("A").with_inline_style(
            TuiStyle::new()
                .flex_basis(Dimension::from(Length::cells(4.0)))
                .flex_shrink(1.0)
                .min_size(flex_width(3.0)),
        ),
        text("B").with_inline_style(
            TuiStyle::new()
                .flex_basis(Dimension::from(Length::cells(4.0)))
                .flex_shrink(1.0),
        ),
    ))
    .with_inline_style(
        flex_fixture_size(4.0, 4.0)
            .display(Display::Flex)
            .align_items(AlignItems::FlexStart),
    )
    .into_view();

    let _growing_terminal = render_layout_root(&growing, 12, 2)?;
    let _shrinking_terminal = render_layout_root(&shrinking, 4, 4)?;

    assert_eq!(
        retained_flex_children(&growing),
        [
            ratatui::layout::Rect::new(0, 0, 4, 1),
            ratatui::layout::Rect::new(4, 0, 8, 1),
        ]
    );
    assert_eq!(
        retained_flex_children(&shrinking),
        [
            ratatui::layout::Rect::new(0, 0, 3, 1),
            ratatui::layout::Rect::new(3, 0, 1, 1),
        ]
    );
    Ok(())
}

/// Verifies align-self overrides container alignment without skipping measurement.
///
/// # Example Under Test
///
/// ```text
/// 8x5 flex row, align-items: flex-start
/// intrinsic probe: 3x2, flex-grow: 1, align-self: flex-end
/// fixed sibling: 2x1
/// ```
///
/// # Assertions
///
/// - The probe is measured at least once for its automatic flex basis.
/// - Growth expands the probe from three to six cells.
/// - Align-self places the probe against the cross-end edge.
/// - The sibling retains the container's cross-start alignment.
#[test]
fn align_self_overrides_parent_alignment_after_intrinsic_measurement() -> Result<()> {
    let (probe, measure_count) = flex_measurement_probe(
        LayoutSize::new(3.0, 2.0),
        TuiStyle::new()
            .flex_grow(1.0)
            .align_self(AlignSelf::FlexEnd),
    );
    let root = div((probe, fixed_flex_child("B", 2.0, 1.0)))
        .with_inline_style(
            flex_fixture_size(8.0, 5.0)
                .display(Display::Flex)
                .align_items(AlignItems::FlexStart),
        )
        .into_view();

    let _terminal = render_layout_root(&root, 8, 5)?;

    assert!(measure_count.get() > 0);
    assert_eq!(
        retained_flex_children(&root),
        [
            ratatui::layout::Rect::new(0, 3, 6, 2),
            ratatui::layout::Rect::new(6, 0, 2, 1),
        ]
    );
    Ok(())
}

/// Verifies replaced and nested widgets participate in flex item sizing.
///
/// # Example Under Test
///
/// ```text
/// image row: intrinsic 4x2 image grows beside a fixed 2x1 item in 10 cells
/// nested row: intrinsic 4-cell flex container grows beside a fixed 2x1 item
/// ```
///
/// # Assertions
///
/// - The image contributes its configured intrinsic size before growing to eight cells.
/// - The nested flex container contributes both intrinsic children before growing.
/// - Nested child geometry remains horizontal and intrinsic after its parent grows.
#[test]
fn replaced_and_nested_widgets_use_intrinsic_flex_bases() -> Result<()> {
    let image_root = div((
        image("asset.png").with_inline_style(
            TuiStyle::new()
                .image_size(TuiSize::new(4, 2))
                .flex_grow(1.0),
        ),
        fixed_flex_child("B", 2.0, 1.0),
    ))
    .with_inline_style(
        flex_fixture_size(10.0, 4.0)
            .display(Display::Flex)
            .align_items(AlignItems::FlexStart),
    )
    .into_view();
    let nested = div((text("AB"), text("CD"))).with_inline_style(
        TuiStyle::new()
            .display(Display::Flex)
            .flex_grow(1.0)
            .align_items(AlignItems::FlexStart),
    );
    let nested_root = div((nested, fixed_flex_child("E", 2.0, 1.0)))
        .with_inline_style(
            flex_fixture_size(10.0, 4.0)
                .display(Display::Flex)
                .align_items(AlignItems::FlexStart),
        )
        .into_view();

    let _image_terminal = render_layout_root(&image_root, 10, 4)?;
    let _nested_terminal = render_layout_root(&nested_root, 10, 4)?;

    assert_eq!(
        retained_flex_children(&image_root),
        [
            ratatui::layout::Rect::new(0, 0, 8, 2),
            ratatui::layout::Rect::new(8, 0, 2, 1),
        ]
    );
    assert_eq!(
        retained_flex_children(&nested_root),
        [
            ratatui::layout::Rect::new(0, 0, 8, 1),
            ratatui::layout::Rect::new(8, 0, 2, 1),
        ]
    );

    let nested_children = nested_root
        .downcast_ref::<leptatui::DivView>()
        .expect("outer Div root")
        .child_views()[0]
        .downcast_ref::<leptatui::DivView>()
        .expect("nested Div child")
        .child_views()
        .iter()
        .map(retained_border_box)
        .collect::<Vec<_>>();
    assert_eq!(
        nested_children,
        [
            ratatui::layout::Rect::new(0, 0, 2, 1),
            ratatui::layout::Rect::new(2, 0, 2, 1),
        ]
    );
    Ok(())
}
