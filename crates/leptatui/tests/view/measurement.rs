/// Measures a view inside a test rendering context.
///
/// # Arguments
///
/// * `view` — View whose intrinsic geometry is requested.
/// * `known_dimensions` — Exact dimensions supplied by parent layout.
/// * `available_space` — Soft constraints for unknown dimensions.
///
/// # Returns
///
/// A [`Result`] containing the measured terminal-cell size.
fn measure_view(
    view: &dyn View,
    known_dimensions: LayoutSize<Option<f32>>,
    available_space: LayoutSize<AvailableSpace>,
) -> leptatui::app::Result<LayoutSize<f32>> {
    let mut terminal = Terminal::new(TestBackend::new(80, 24))?;
    let mut measured = LayoutSize::all(0.0);
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        measured = view.measure(known_dimensions, available_space, &mut ctx);
    })?;
    Ok(measured)
}

/// Custom leaf that records measurement inputs independently of rendering.
struct MeasurementProbe {
    /// Number of times painting was requested.
    render_count: Rc<Cell<usize>>,
    /// Most recent exact dimensions received by measurement.
    known_dimensions: Rc<RefCell<Option<LayoutSize<Option<f32>>>>>,
    /// Most recent available-space constraints received by measurement.
    available_space: Rc<RefCell<Option<LayoutSize<AvailableSpace>>>>,
}

impl View for MeasurementProbe {
    fn render(&self, _ctx: &mut RenderCtx<'_, '_>) -> leptatui::app::Result<()> {
        self.render_count
            .set(self.render_count.get().saturating_add(1));
        Ok(())
    }

    fn measure(
        &self,
        known_dimensions: LayoutSize<Option<f32>>,
        available_space: LayoutSize<AvailableSpace>,
        _ctx: &mut RenderCtx<'_, '_>,
    ) -> LayoutSize<f32> {
        self.known_dimensions.replace(Some(known_dimensions));
        self.available_space.replace(Some(available_space));
        LayoutSize::new(
            known_dimensions.width.unwrap_or(7.0),
            known_dimensions.height.unwrap_or(2.0),
        )
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Verifies custom views receive both measurement constraints without painting.
///
/// # Example Under Test
///
/// ```text
/// known dimensions = 11 × unknown
/// available space = definite 20 × min-content
/// ```
///
/// # Assertions
///
/// - The custom view receives both exact-dimension values.
/// - The custom view receives both available-space constraints.
/// - The exact width overrides the probe's intrinsic width.
/// - Measurement does not call the view's render method.
#[test]
fn custom_view_measurement_receives_constraints_without_rendering() -> leptatui::app::Result<()> {
    let render_count = Rc::new(Cell::new(0));
    let seen_known = Rc::new(RefCell::new(None));
    let seen_available = Rc::new(RefCell::new(None));
    let view = MeasurementProbe {
        render_count: Rc::clone(&render_count),
        known_dimensions: Rc::clone(&seen_known),
        available_space: Rc::clone(&seen_available),
    };
    let known = LayoutSize::new(Some(11.0), None);
    let available = LayoutSize::new(
        AvailableSpace::Definite(20.0),
        AvailableSpace::MinContent,
    );

    let measured = measure_view(&view, known, available)?;

    assert_eq!(measured, LayoutSize::new(11.0, 2.0));
    assert_eq!(*seen_known.borrow(), Some(known));
    assert_eq!(*seen_available.borrow(), Some(available));
    assert_eq!(render_count.get(), 0);
    Ok(())
}

/// Verifies text reports min-content, max-content, constrained, and zero sizes.
///
/// # Example Under Test
///
/// ```text
/// "wide word"
/// min-content, max-content, definite width 5, and known size 0 × 0
/// ```
///
/// # Assertions
///
/// - Min-content uses the longest unbreakable word and wraps to two rows.
/// - Max-content uses the complete hard line and stays on one row.
/// - A definite width between those bounds wraps without changing the constraint.
/// - Exact zero dimensions are honored.
#[test]
fn text_measurement_supports_intrinsic_and_definite_constraints() -> leptatui::app::Result<()> {
    let view = text("wide word");
    let unknown = LayoutSize::all(None);

    let min_content = measure_view(
        &view,
        unknown,
        LayoutSize::all(AvailableSpace::MinContent),
    )?;
    let max_content = measure_view(
        &view,
        unknown,
        LayoutSize::all(AvailableSpace::MaxContent),
    )?;
    let definite = measure_view(
        &view,
        unknown,
        LayoutSize::new(
            AvailableSpace::Definite(5.0),
            AvailableSpace::MaxContent,
        ),
    )?;
    let zero = measure_view(
        &view,
        LayoutSize::new(Some(0.0), Some(0.0)),
        LayoutSize::all(AvailableSpace::Definite(0.0)),
    )?;

    assert_eq!(min_content, LayoutSize::new(4.0, 2.0));
    assert_eq!(max_content, LayoutSize::new(9.0, 1.0));
    assert_eq!(definite, LayoutSize::new(5.0, 2.0));
    assert_eq!(zero, LayoutSize::all(0.0));
    Ok(())
}

/// Verifies terminal-cell width handles wide Unicode glyphs.
///
/// # Example Under Test
///
/// ```text
/// "界 a"
/// ```
///
/// # Assertions
///
/// - Min-content width is the two-cell wide glyph.
/// - Max-content width includes the glyph, space, and ASCII character.
#[test]
fn text_measurement_uses_unicode_terminal_width() -> leptatui::app::Result<()> {
    let view = text("界 a");
    let unknown = LayoutSize::all(None);

    let min_content = measure_view(
        &view,
        unknown,
        LayoutSize::all(AvailableSpace::MinContent),
    )?;
    let max_content = measure_view(
        &view,
        unknown,
        LayoutSize::all(AvailableSpace::MaxContent),
    )?;

    assert_eq!(min_content.width, 2.0);
    assert_eq!(max_content.width, 4.0);
    Ok(())
}

/// Verifies controls, editors, images, and code return both intrinsic axes.
///
/// # Example Under Test
///
/// ```text
/// button("Go")
/// input("Ada")
/// text_area("Ada\n")
/// image("asset.png") sized 9 × 4
/// code_block("abcdef") constrained to width 4
/// ```
///
/// # Assertions
///
/// - Bordered controls include their effective chrome.
/// - Text areas count trailing blank rows.
/// - Images honor configured terminal-cell dimensions.
/// - Narrow code blocks wrap source while preserving their known width.
#[test]
fn specialized_leaf_views_measure_width_and_height() -> leptatui::app::Result<()> {
    let unknown = LayoutSize::all(None);
    let max_content = LayoutSize::all(AvailableSpace::MaxContent);
    let button_size = measure_view(&button("Go"), unknown, max_content)?;
    let input_size = measure_view(&input("Ada"), unknown, max_content)?;
    let text_area_size = measure_view(&text_area("Ada\n"), unknown, max_content)?;
    let image_size = measure_view(
        &image("asset.png").with_inline_style(TuiStyle::new().image_size(TuiSize::new(9, 4))),
        unknown,
        max_content,
    )?;
    let code_size = measure_view(
        &code_block("abcdef"),
        LayoutSize::new(Some(4.0), None),
        LayoutSize::all(AvailableSpace::Definite(4.0)),
    )?;

    assert_eq!(button_size, LayoutSize::new(4.0, 3.0));
    assert_eq!(input_size, LayoutSize::new(5.0, 3.0));
    assert_eq!(text_area_size, LayoutSize::new(5.0, 4.0));
    assert_eq!(image_size, LayoutSize::new(9.0, 4.0));
    assert_eq!(code_size, LayoutSize::new(4.0, 5.0));
    Ok(())
}

/// Verifies semantic lists include marker columns in intrinsic geometry.
///
/// # Example Under Test
///
/// ```text
/// - short
/// - long word
/// ```
///
/// # Assertions
///
/// - Min-content includes the marker gap and longest unbreakable word.
/// - Max-content includes the marker gap and complete preferred line.
/// - Min-content wrapping produces more rows than max-content.
#[test]
fn list_measurement_includes_markers_and_nested_content_widths() -> leptatui::app::Result<()> {
    let view = unordered_list([
        list_item([paragraph("short")]),
        list_item([paragraph("long word")]),
    ]);
    let unknown = LayoutSize::all(None);

    let min_content = measure_view(
        &view,
        unknown,
        LayoutSize::all(AvailableSpace::MinContent),
    )?;
    let max_content = measure_view(
        &view,
        unknown,
        LayoutSize::all(AvailableSpace::MaxContent),
    )?;

    assert_eq!(min_content.width, 7.0);
    assert_eq!(max_content.width, 11.0);
    assert!(min_content.height > max_content.height);
    Ok(())
}

/// Verifies semantic tables expose intrinsic column widths and constrained height.
///
/// # Example Under Test
///
/// ```text
/// table row: ["Name", "Build status"]
/// ```
///
/// # Assertions
///
/// - Min-content uses the widest unbreakable word in each column.
/// - Max-content uses each column's preferred hard-line width.
/// - A narrow known width is retained and increases wrapped table height.
#[test]
fn table_measurement_supports_intrinsic_and_narrow_widths() -> leptatui::app::Result<()> {
    let view = table([table_body([table_row([
        table_cell("Name"),
        table_cell("Build status"),
    ])])]);
    let unknown = LayoutSize::all(None);
    let min_content = measure_view(
        &view,
        unknown,
        LayoutSize::all(AvailableSpace::MinContent),
    )?;
    let max_content = measure_view(
        &view,
        unknown,
        LayoutSize::all(AvailableSpace::MaxContent),
    )?;
    let narrow = measure_view(
        &view,
        LayoutSize::new(Some(8.0), None),
        LayoutSize::all(AvailableSpace::Definite(8.0)),
    )?;

    assert_eq!(min_content.width, 13.0);
    assert_eq!(max_content.width, 19.0);
    assert_eq!(narrow.width, 8.0);
    assert!(narrow.height > max_content.height);
    Ok(())
}
