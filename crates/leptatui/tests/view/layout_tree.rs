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

/// Styleable leaf that records the geometry exposed by its render context.
struct GeometryContextProbe {
    /// Selector and retained layout metadata.
    metadata: StyleMetadata,
    /// Last geometry observed during painting.
    geometry: Rc<Cell<Option<LayoutGeometry>>>,
}

impl View for GeometryContextProbe {
    /// Records the active rounded layout snapshot.
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        self.geometry.set(Some(ctx.layout_geometry()));
        Ok(())
    }

    /// Returns a minimal intrinsic size before authored constraints apply.
    fn measure(
        &self,
        _known_dimensions: LayoutSize<Option<f32>>,
        _available_space: LayoutSize<AvailableSpace>,
        _ctx: &mut RenderCtx<'_, '_>,
    ) -> LayoutSize<f32> {
        LayoutSize::all(1.0)
    }

    /// Returns the probe's selector and retained layout metadata.
    fn style_metadata(&self) -> Option<&StyleMetadata> {
        Some(&self.metadata)
    }

    /// Returns mutable selector and retained layout metadata.
    fn style_metadata_mut(&mut self) -> Option<&mut StyleMetadata> {
        Some(&mut self.metadata)
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

/// Verifies a nested layout uses its assigned area as its containing block.
///
/// # Example Under Test
///
/// ```text
/// 12x3 bordered block
/// 100%-wide inner flex layout: A <space-between> B
/// ```
///
/// # Assertions
///
/// - The first flex child renders against the block's left inner edge.
/// - The second flex child renders against the block's right inner edge.
#[test]
fn nested_layout_uses_assigned_area_as_containing_block() -> Result<()> {
    let root = block(
        div((text("A"), text("B"))).with_inline_style(
            TuiStyle::new()
                .display(Display::Flex)
                .size(LayoutSize::new(
                    Dimension::from(Length::percent(100.0)),
                    Dimension::Auto,
                ))
                .justify_content(JustifyContent::SpaceBetween),
        ),
    )
    .into_view();

    let terminal = render_layout_root(&root, 12, 3)?;

    assert_eq!(cell_symbol(&terminal, 1, 1, 12), "A");
    assert_eq!(cell_symbol(&terminal, 10, 1, 12), "B");
    Ok(())
}

/// Verifies positioned siblings paint in ascending z-index order.
///
/// # Example Under Test
///
/// ```text
/// 1x1 relative container
/// absolute "A": z-index 1
/// absolute "B": z-index 0, later in source order
/// ```
///
/// # Assertions
///
/// - The higher-z-index first sibling paints over the later sibling.
#[test]
fn positioned_siblings_paint_by_z_index() -> Result<()> {
    let inset = Edges::new(
        Length::cells(0.0).into(),
        LengthAuto::Auto,
        LengthAuto::Auto,
        Length::cells(0.0).into(),
    );
    let higher = text("A").with_inline_style(
        TuiStyle::new()
            .position(Position::Absolute)
            .inset(inset)
            .z_index(ZIndex::Integer(1)),
    );
    let lower = text("B").with_inline_style(
        TuiStyle::new()
            .position(Position::Absolute)
            .inset(inset)
            .z_index(ZIndex::Integer(0)),
    );
    let root = div((higher, lower))
        .with_inline_style(
            TuiStyle::new()
                .position(Position::Relative)
                .size(LayoutSize::all(Dimension::from(Length::cells(1.0)))),
        )
        .into_view();

    let terminal = render_layout_root(&root, 1, 1)?;

    assert_eq!(cell_symbol(&terminal, 0, 0, 1), "A");
    Ok(())
}

/// Verifies pointer focus follows z-index rather than source order.
///
/// # Example Under Test
///
/// ```text
/// 6x3 relative container
/// two overlapping absolute buttons with z-index 0 and 1
/// test both possible source orders
/// ```
///
/// # Assertions
///
/// - The later high-z-index button receives pointer focus over an earlier sibling.
/// - The earlier high-z-index button receives pointer focus over a later sibling.
/// - Flattened button focus state remains source-ordered.
///
/// # Why
///
/// Pointer interaction should target the frontmost painted control without
/// changing keyboard traversal order.
#[test]
fn positioned_pointer_focus_follows_recorded_z_index_paint_order() -> Result<()> {
    let inset = Edges::new(
        Length::cells(0.0).into(),
        LengthAuto::Auto,
        LengthAuto::Auto,
        Length::cells(0.0).into(),
    );
    let button_style = |z_index| {
        TuiStyle::new()
            .box_sizing(BoxSizing::BorderBox)
            .position(Position::Absolute)
            .inset(inset)
            .z_index(ZIndex::Integer(z_index))
            .size(LayoutSize::new(
                Dimension::from(Length::cells(6.0)),
                Dimension::from(Length::cells(3.0)),
            ))
    };
    let container_style = TuiStyle::new()
        .box_sizing(BoxSizing::BorderBox)
        .position(Position::Relative)
        .size(LayoutSize::new(
            Dimension::from(Length::cells(6.0)),
            Dimension::from(Length::cells(3.0)),
        ));

    let mut later_is_higher = div((
        button("Low").with_inline_style(button_style(0)),
        button("High").with_inline_style(button_style(1)),
    ))
    .with_inline_style(container_style)
    .into_view();
    let _terminal = render_layout_root(&later_is_higher, 6, 3)?;
    assert!(later_is_higher.__focus_control_at_position(2, 1));
    assert_eq!(button_focuses(later_is_higher.as_view()), vec![false, true]);

    let mut earlier_is_higher = div((
        button("High").with_inline_style(button_style(1)),
        button("Low").with_inline_style(button_style(0)),
    ))
    .with_inline_style(container_style)
    .into_view();
    let _terminal = render_layout_root(&earlier_is_higher, 6, 3)?;
    assert!(earlier_is_higher.__focus_control_at_position(2, 1));
    assert_eq!(
        button_focuses(earlier_is_higher.as_view()),
        vec![true, false]
    );
    Ok(())
}

/// Verifies zero-valued grid placements safely use automatic placement.
///
/// # Example Under Test
///
/// ```text
/// grid container
/// child 1 grid row starts at line 0
/// child 2 grid column ends with span 0
/// ```
///
/// # Assertions
///
/// - Rendering does not panic for a zero grid line.
/// - Rendering does not panic for a zero grid span.
/// - Both children remain visible through automatic placement.
///
/// # Why
///
/// Invalid public layout values should degrade predictably at the layout-engine
/// boundary rather than panic inside the engine.
#[test]
fn zero_grid_placements_fall_back_to_automatic_layout() -> Result<()> {
    let first = text("A").with_inline_style(
        TuiStyle::new().grid_row(GridLine::new(
            GridPlacement::line(0),
            GridPlacement::Auto,
        )),
    );
    let second = text("B").with_inline_style(
        TuiStyle::new().grid_column(GridLine::new(
            GridPlacement::Auto,
            GridPlacement::span(0),
        )),
    );
    let root = div((first, second))
        .with_inline_style(
            TuiStyle::new()
                .display(Display::Grid)
                .size(LayoutSize::new(
                    Dimension::from(Length::cells(4.0)),
                    Dimension::from(Length::cells(2.0)),
                )),
        )
        .into_view();

    let terminal = render_layout_root(&root, 4, 2)?;

    assert!(symbol_position_opt(&terminal, "A", 4).is_some());
    assert!(symbol_position_opt(&terminal, "B", 4).is_some());
    Ok(())
}

/// Verifies one rounded snapshot exposes all paint and interaction rectangles.
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
/// - A box without scrollbars uses its content box as the viewport.
/// - The child inherits the root terminal clip.
#[test]
fn layout_tree_retains_rounded_box_geometry() -> Result<()> {
    let styled_block = block(text("inside")).with_inline_style(
        TuiStyle::new()
            .box_sizing(BoxSizing::BorderBox)
            .size(LayoutSize::new(
                Dimension::from(Length::cells(6.0)),
                Dimension::from(Length::cells(6.0)),
            ))
            .overflow(Axes::all(Overflow::Visible))
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
    assert_eq!(layout.viewport, ratatui::layout::Rect::new(2, 2, 2, 2));
    assert_eq!(layout.clip, ratatui::layout::Rect::new(0, 0, 20, 10));
    Ok(())
}

/// Verifies render contexts expose the translated retained snapshot.
///
/// # Example Under Test
///
/// ```text
/// 6x5 custom leaf
/// borders: all
/// padding: 1
/// terminal: 10x6
/// ```
///
/// # Assertions
///
/// - The custom view receives the same geometry retained on its metadata.
/// - Border, padding, content, viewport, and clip rectangles are all exposed.
#[test]
fn render_context_exposes_retained_layout_geometry() -> Result<()> {
    let observed = Rc::new(Cell::new(None));
    let mut metadata = StyleMetadata::new(ViewType::new("GeometryContextProbe"));
    metadata.set_inline_style(
        TuiStyle::new()
            .borders(Borders::ALL)
            .box_sizing(BoxSizing::BorderBox)
            .size(LayoutSize::new(
                Dimension::from(Length::cells(6.0)),
                Dimension::from(Length::cells(5.0)),
            ))
            .padding(TuiSpacing::uniform(1)),
    );
    let root = div((GeometryContextProbe {
        metadata,
        geometry: Rc::clone(&observed),
    },))
    .into_view();

    let _terminal = render_layout_root(&root, 10, 6)?;
    let geometry = observed.get().expect("render context geometry");

    assert_eq!(geometry.border_box, ratatui::layout::Rect::new(0, 0, 6, 5));
    assert_eq!(geometry.padding_box, ratatui::layout::Rect::new(1, 1, 4, 3));
    assert_eq!(geometry.content_box, ratatui::layout::Rect::new(2, 2, 2, 1));
    assert_eq!(geometry.viewport, geometry.content_box);
    assert_eq!(geometry.clip, ratatui::layout::Rect::new(0, 0, 10, 6));
    Ok(())
}

/// Verifies content-box sizing adds authored padding and borders.
///
/// # Example Under Test
///
/// ```text
/// content size: 6x6
/// padding: 1 cell
/// borders: 1 cell
/// ```
///
/// # Assertions
///
/// - The border box includes content, padding, and borders as `10x10`.
/// - Removing the borders produces an `8x8` padding box.
/// - Removing the padding produces the authored `6x6` content box.
#[test]
fn content_box_sizing_includes_builtin_chrome() -> Result<()> {
    let root = block(text("inside"))
        .with_inline_style(
            TuiStyle::new()
                .size(LayoutSize::new(
                    Dimension::from(Length::cells(6.0)),
                    Dimension::from(Length::cells(6.0)),
                ))
                .padding(TuiSpacing::uniform(1)),
        )
        .into_view();

    let _terminal = render_layout_root(&root, 20, 15)?;

    let layout = root
        .style_metadata()
        .and_then(StyleMetadata::layout_geometry)
        .expect("computed block geometry");
    assert_eq!(layout.border_box, ratatui::layout::Rect::new(0, 0, 10, 10));
    assert_eq!(layout.padding_box, ratatui::layout::Rect::new(1, 1, 8, 8));
    assert_eq!(layout.content_box, ratatui::layout::Rect::new(2, 2, 6, 6));
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
