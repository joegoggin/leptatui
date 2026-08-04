/// Verifies editable builders return distinct concrete view types.
///
/// # Example Under Test
///
/// ```text
/// input("Ada")
/// text_area("Notes")
/// ```
///
/// # Assertions
///
/// - `input` returns an [`InputView`].
/// - `text_area` returns a [`TextAreaView`].
/// - Type-erased inputs compare equal to identical inputs.
/// - Type-erased text areas compare equal to identical text areas.
/// - Inputs and text areas do not compare equal after type erasure.
#[test]
fn editable_builders_return_distinct_concrete_types() {
    let input_view: InputView = input("Ada");
    let text_area_view: TextAreaView = text_area("Notes");

    assert_eq!(input_view.into_view(), input("Ada").into_view());
    assert_eq!(text_area_view.into_view(), text_area("Notes").into_view());
    assert_ne!(input("same").into_view(), text_area("same").into_view());
}

/// Verifies view builders store default selector metadata.
///
/// # Example Under Test
///
/// ```text
/// block(text("child"))
/// ```
///
/// # Assertions
///
/// - Block metadata is available.
/// - The view type is `Block`.
/// - The metadata has no id.
/// - The metadata has no classes.
/// - The metadata has no inline style.
/// - The metadata is not focused.
/// - The metadata is not active.
/// - The metadata is not in insert mode.
/// - The metadata is not in visual mode.
/// - The metadata has not been visited.
#[test]
fn view_builders_store_default_selector_metadata() {
    let block_view = block(text("child"));
    let metadata = block_view.style_metadata().expect("block metadata");

    assert_eq!(metadata.view_type(), ViewType::Block);
    assert_eq!(metadata.id(), None);
    assert!(metadata.classes().is_empty());
    assert_eq!(metadata.inline_style(), None);
    assert!(!metadata.is_focused());
    assert!(!metadata.is_active());
    assert!(!metadata.is_insert());
    assert!(!metadata.is_visual());
    assert!(!metadata.is_visited());
}

/// Verifies view metadata setters store selector fields.
///
/// # Example Under Test
///
/// ```text
/// button("Save")
///     .with_id("save")
///     .with_classes("primary toolbar")
///     .with_inline_style(yellow)
///     .with_focus(true)
///     .with_active(true)
///     .with_insert(true)
///     .with_visual(true)
///     .with_visited(true)
/// ```
///
/// # Assertions
///
/// - Button metadata is available.
/// - The view type is `Button`.
/// - The metadata id is `save`.
/// - The metadata classes are `primary` and `toolbar`.
/// - The metadata inline style is yellow.
/// - The metadata is focused.
/// - The metadata is active.
/// - The metadata is in insert mode.
/// - The metadata is in visual mode.
/// - The metadata is visited.
#[test]
fn view_metadata_setters_store_selector_fields() {
    let style = TuiStyle::new().foreground(Color::Yellow);
    let view = button("Save")
        .with_id("save")
        .with_classes("primary toolbar")
        .with_inline_style(style.clone())
        .with_focus(true)
        .with_active(true)
        .with_insert(true)
        .with_visual(true)
        .with_visited(true);
    let metadata = view.style_metadata().expect("button metadata");

    assert_eq!(metadata.view_type(), ViewType::Button);
    assert_eq!(metadata.id(), Some("save"));
    assert_eq!(
        metadata.classes(),
        &[String::from("primary"), String::from("toolbar")]
    );
    assert_eq!(metadata.inline_style(), Some(style));
    assert!(metadata.is_focused());
    assert!(metadata.is_active());
    assert!(metadata.is_insert());
    assert!(metadata.is_visual());
    assert!(metadata.is_visited());
}

/// Verifies editable metadata follows retained Vim insert mode.
///
/// # Example Under Test
///
/// ```text
/// input("Ada"): Normal -> Insert -> Normal
/// ```
///
/// # Assertions
///
/// - A new input does not match `:insert`.
/// - Entering insert mode makes its metadata match `:insert`.
/// - Returning to normal mode clears the insert state.
#[test]
fn editable_metadata_tracks_retained_insert_mode() {
    let mut view = input("Ada");
    assert!(!view.metadata().is_insert());

    view.editable_state_mut().set_mode(VimMode::Insert);
    assert!(view.metadata().is_insert());

    view.editable_state_mut().set_mode(VimMode::Normal);
    assert!(!view.metadata().is_insert());
}

/// Verifies editable metadata follows both retained Vim visual modes.
///
/// # Example Under Test
///
/// ```text
/// input("Ada"): Normal -> Visual -> VisualLine -> Normal
/// ```
///
/// # Assertions
///
/// - A new input does not match `:visual`.
/// - Character-wise visual mode makes its metadata match `:visual`.
/// - Visual-line mode continues to match `:visual`.
/// - Returning to normal mode clears the visual state.
#[test]
fn editable_metadata_tracks_retained_visual_modes() {
    let mut view = input("Ada");
    assert!(!view.metadata().is_visual());

    view.editable_state_mut().set_mode(VimMode::Visual);
    assert!(view.metadata().is_visual());

    view.editable_state_mut().set_mode(VimMode::VisualLine);
    assert!(view.metadata().is_visual());

    view.editable_state_mut().set_mode(VimMode::Normal);
    assert!(!view.metadata().is_visual());
}
