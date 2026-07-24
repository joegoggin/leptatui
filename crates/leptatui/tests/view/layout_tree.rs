/// Renders an erased root through the layout-aware entrypoint.
///
/// # Arguments
///
/// * `root` — Erased view root to lay out and paint.
/// * `width` — Test terminal width.
/// * `height` — Test terminal height.
///
/// # Returns
///
/// A [`Terminal`] containing the rendered output.
///
/// # Errors
///
/// Returns [`leptatui::Error::Io`] if terminal drawing or view rendering fails.
fn render_layout_root(
    root: &AnyView,
    width: u16,
    height: u16,
) -> Result<Terminal<TestBackend>> {
    let mut terminal = Terminal::new(TestBackend::new(width, height))?;
    let mut render_result = Ok(());
    terminal.draw(|frame| {
        let mut ctx = RenderCtx::new(frame);
        render_result = root.render(&mut ctx);
    })?;
    render_result?;
    Ok(terminal)
}

/// Styleable leaf that records layout, painting, and custom-event calls.
struct HiddenLayoutProbe {
    /// Selector and layout metadata.
    metadata: StyleMetadata,
    /// Number of intrinsic measurements requested.
    measures: Rc<Cell<usize>>,
    /// Number of paint calls requested.
    renders: Rc<Cell<usize>>,
    /// Number of custom events received.
    events: Rc<Cell<usize>>,
}

impl View for HiddenLayoutProbe {
    /// Records an unexpected paint request.
    fn render(&self, _ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        self.renders
            .set(self.renders.get().saturating_add(1));
        Ok(())
    }

    /// Records an unexpected intrinsic measurement request.
    fn measure(
        &self,
        _known_dimensions: LayoutSize<Option<f32>>,
        _available_space: LayoutSize<AvailableSpace>,
        _ctx: &mut RenderCtx<'_, '_>,
    ) -> LayoutSize<f32> {
        self.measures
            .set(self.measures.get().saturating_add(1));
        LayoutSize::all(1.0)
    }

    /// Returns the probe's selector and layout metadata.
    fn style_metadata(&self) -> Option<&StyleMetadata> {
        Some(&self.metadata)
    }

    /// Returns mutable selector and layout metadata.
    fn style_metadata_mut(&mut self) -> Option<&mut StyleMetadata> {
        Some(&mut self.metadata)
    }

    /// Records an unexpected custom event.
    fn on_event(&mut self, _event: &Event) -> Result<AppControl> {
        self.events.set(self.events.get().saturating_add(1));
        Ok(AppControl::Continue)
    }

    /// Returns the probe for shared concrete-type inspection.
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    /// Returns the probe for mutable concrete-type inspection.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Verifies one rounded snapshot exposes border, padding, and content boxes.
///
/// # Example Under Test
///
/// ```text
/// 6x6 border-box block with one-cell borders and one-cell padding
/// ```
///
/// # Assertions
///
/// - The border box occupies the authored `6x6` area.
/// - Removing one-cell borders produces a `4x4` padding box.
/// - Removing one-cell padding produces a `2x2` content box.
#[test]
fn layout_tree_retains_rounded_box_geometry() -> Result<()> {
    let styled_block = block(text("inside")).with_inline_style(
        TuiStyle::new()
            .box_sizing(BoxSizing::BorderBox)
            .size(LayoutSize::new(
                Dimension::from(Length::cells(6.0)),
                Dimension::from(Length::cells(6.0)),
            ))
            .padding(TuiSpacing::uniform(1)),
    );
    let root = div((styled_block, text("after"))).into_view();

    let _terminal = render_layout_root(&root, 20, 10)?;

    let layout = root
        .downcast_ref::<leptatui::DivView>()
        .expect("Div root")
        .child_views()[0]
        .style_metadata()
        .expect("block metadata")
        .layout_geometry()
        .expect("computed block geometry");
    assert_eq!(layout.border_box, ratatui::layout::Rect::new(0, 0, 6, 6));
    assert_eq!(layout.padding_box, ratatui::layout::Rect::new(1, 1, 4, 4));
    assert_eq!(layout.content_box, ratatui::layout::Rect::new(2, 2, 2, 2));
    Ok(())
}

/// Verifies viewport-relative values are recomputed after terminal resize.
///
/// # Example Under Test
///
/// ```text
/// width: 50vw
/// height: 25vh
/// viewport: 20x12, then 40x20
/// ```
///
/// # Assertions
///
/// - The first render retains a `10x3` border box.
/// - The second render replaces it with a `20x5` border box.
#[test]
fn layout_tree_rebuilds_viewport_geometry_on_resize() -> Result<()> {
    let root = text("responsive")
        .with_inline_style(
            TuiStyle::new()
                .box_sizing(BoxSizing::BorderBox)
                .size(LayoutSize::new(
                    Dimension::from(Length::vw(50.0)),
                    Dimension::from(Length::vh(25.0)),
                )),
        )
        .into_view();

    let _small = render_layout_root(&root, 20, 12)?;
    assert_eq!(
        root.style_metadata()
            .and_then(StyleMetadata::layout_geometry)
            .expect("small geometry")
            .border_box,
        ratatui::layout::Rect::new(0, 0, 10, 3)
    );

    let _large = render_layout_root(&root, 40, 20)?;
    assert_eq!(
        root.style_metadata()
            .and_then(StyleMetadata::layout_geometry)
            .expect("large geometry")
            .border_box,
        ratatui::layout::Rect::new(0, 0, 20, 5)
    );
    Ok(())
}

/// Verifies hidden subtrees are omitted from painting and focus traversal.
///
/// # Example Under Test
///
/// ```text
/// div((hidden focused button, visible button))
/// Hidden { display: none }
/// ```
///
/// # Assertions
///
/// - The hidden button has no retained geometry.
/// - The hidden label is absent from the terminal.
/// - Focus traversal counts only the visible button.
/// - Enter does not activate the hidden focused button.
#[test]
fn display_none_excludes_painting_focus_and_activation() -> Result<()> {
    let activations = Rc::new(Cell::new(0usize));
    let action_count = Rc::clone(&activations);
    let hidden = button("Hidden")
        .on_press(move || {
            action_count.set(action_count.get().saturating_add(1));
            AppControl::Continue
        })
        .with_focus(true)
        .with_inline_style(TuiStyle::new().display(Display::None));
    let root = div((hidden, button("Visible"))).into_view();

    let terminal = render_layout_root(&root, 20, 4)?;

    let children = root
        .downcast_ref::<leptatui::DivView>()
        .expect("Div root")
        .child_views();
    assert!(
        children[0]
            .style_metadata()
            .and_then(StyleMetadata::layout_geometry)
            .is_none()
    );
    assert!(!rendered_text(&terminal).contains("Hidden"));
    assert_eq!(root.__focusable_count(), 1);

    let mut root = root;
    assert_eq!(
        root.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?,
        KeyControl::Pass
    );
    assert_eq!(activations.get(), 0);
    Ok(())
}

/// Verifies hidden leaves are not measured, painted, or sent custom events.
///
/// # Example Under Test
///
/// ```text
/// HiddenLayoutProbe { display: none }
/// root render followed by Event::FocusGained
/// ```
///
/// # Assertions
///
/// - Taffy does not request intrinsic measurement for the hidden leaf.
/// - Painting does not invoke the hidden leaf.
/// - Custom event dispatch does not invoke the hidden leaf.
#[test]
fn display_none_skips_measurement_paint_and_custom_events() -> Result<()> {
    let measures = Rc::new(Cell::new(0usize));
    let renders = Rc::new(Cell::new(0usize));
    let events = Rc::new(Cell::new(0usize));
    let mut metadata = StyleMetadata::new(ViewType::new("HiddenLayoutProbe"));
    metadata.set_inline_style(TuiStyle::new().display(Display::None));
    let probe = HiddenLayoutProbe {
        metadata,
        measures: Rc::clone(&measures),
        renders: Rc::clone(&renders),
        events: Rc::clone(&events),
    };
    let root = div((probe, text("Visible"))).into_view();

    let _terminal = render_layout_root(&root, 20, 4)?;
    let mut root = root;
    assert_eq!(
        root.handle_event(Event::FocusGained)?,
        AppControl::Continue
    );

    assert_eq!(measures.get(), 0);
    assert_eq!(renders.get(), 0);
    assert_eq!(events.get(), 0);
    Ok(())
}

/// Verifies dynamic content is materialized once for layout and reused by paint.
///
/// # Example Under Test
///
/// ```text
/// dynamic(|| text("Dynamic"))
/// two root renders
/// ```
///
/// # Assertions
///
/// - The dynamic factory runs once during the first render.
/// - The dynamic factory runs exactly once more during the second render.
#[test]
fn dynamic_layout_child_is_refreshed_once_per_render() -> Result<()> {
    let builds = Rc::new(Cell::new(0usize));
    let build_count = Rc::clone(&builds);
    let root = dynamic(move || {
        build_count.set(build_count.get().saturating_add(1));
        text("Dynamic")
    })
    .into_view();

    let _first = render_layout_root(&root, 20, 2)?;
    assert_eq!(builds.get(), 1);

    let _second = render_layout_root(&root, 20, 2)?;
    assert_eq!(builds.get(), 2);
    Ok(())
}
