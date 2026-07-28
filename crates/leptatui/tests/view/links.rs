/// Verifies the link builder retains rich text, target classification, and metadata.
///
/// # Example Under Test
///
/// ```text
/// link(Text::from("Guide"), "https://example.com")
/// ```
///
/// # Assertions
///
/// - The builder stores the supplied rich text.
/// - The string destination is classified as an absolute URL.
/// - Selector metadata uses `ViewType::Link`.
#[test]
fn link_builder_stores_rich_text_target_and_metadata() {
    let label = Text::from(Line::from(vec![
        Span::raw("Gui"),
        Span::styled("de", Style::new().fg(Color::Yellow)),
    ]));
    let view = link(label.clone(), "https://example.com");

    assert_eq!(view.content(), &label);
    assert_eq!(
        view.target(),
        &LinkTarget::Url("https://example.com".to_owned())
    );
    assert_eq!(view.metadata().view_type(), ViewType::Link);
}

/// Verifies links render recognizable default and focused states.
///
/// # Example Under Test
///
/// ```text
/// Link("Guide", "https://example.com")
/// render, Tab, render
/// ```
///
/// # Assertions
///
/// - The default link style is underlined but not reversed.
/// - The focused link style is both underlined and reversed.
#[test]
fn link_renders_default_and_focused_styles() -> leptatui::app::Result<()> {
    let mut view = link("Guide", "https://example.com");
    let mut terminal = Terminal::new(TestBackend::new(8, 1))?;
    draw_view(&mut terminal, &view)?;
    assert!(cell_modifiers(&terminal, 0, 0, 8).contains(Modifier::UNDERLINED));
    assert!(!cell_modifiers(&terminal, 0, 0, 8).contains(Modifier::REVERSED));

    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Tab))?,
        KeyControl::Handled
    );
    draw_view(&mut terminal, &view)?;
    let focused = cell_modifiers(&terminal, 0, 0, 8);
    assert!(focused.contains(Modifier::UNDERLINED));
    assert!(focused.contains(Modifier::REVERSED));
    Ok(())
}

/// Verifies inactive fragments are skipped and link-open failures propagate.
///
/// # Example Under Test
///
/// ```text
/// div((link("Fragment", "#part"), link("Missing", missing_path)))
/// Tab, Enter
/// ```
///
/// # Assertions
///
/// - The fragment-only link is excluded from focus traversal.
/// - Focus moves to the missing filesystem target.
/// - Activating that target returns a link-open error.
#[test]
fn link_focus_skips_fragments_and_propagates_open_errors() -> leptatui::app::Result<()> {
    let missing = std::env::temp_dir().join(format!(
        "leptatui-missing-link-target-{}",
        std::process::id()
    ));
    let mut view = div((link("Fragment", "#part"), link("Missing", missing)));
    assert_eq!(view.__focusable_count(), 1);
    assert_eq!(
        view.handle_key_event(key_event(KeyCode::Tab))?,
        KeyControl::Handled
    );
    assert_eq!(control_focuses(&view), vec![false, true]);

    let error = view
        .handle_key_event(key_event(KeyCode::Enter))
        .expect_err("missing link target should fail");
    assert!(matches!(error, leptatui::Error::LinkOpen { .. }));
    Ok(())
}

/// Verifies standalone link focus survives matching view reconciliation.
///
/// # Example Under Test
///
/// ```text
/// previous = Link("Guide", "https://example.com"), focused
/// matching = Link("Updated", "https://example.com")
/// different = Link("Other", "https://example.org")
/// ```
///
/// # Assertions
///
/// - Reconciliation preserves focus when the link target is unchanged.
/// - Reconciliation clears focus when the link target changes.
#[test]
fn link_reconciliation_retains_focus_only_for_matching_targets() -> leptatui::app::Result<()> {
    let mut previous = link("Guide", "https://example.com");
    previous.handle_key_event(key_event(KeyCode::Tab))?;
    assert_eq!(control_focuses(&previous), vec![true]);

    let mut matching = link("Updated", "https://example.com");
    leptatui::__private::__reconcile_view(&mut matching, &previous);
    assert_eq!(control_focuses(&matching), vec![true]);

    let mut different = link("Other", "https://example.org");
    leptatui::__private::__reconcile_view(&mut different, &previous);
    assert_eq!(control_focuses(&different), vec![false]);
    Ok(())
}
