//! Explorer page destination tests.

use super::*;

/// Verifies explicit page destinations do not depend on route history.
///
/// # Example Under Test
///
/// ```text
/// Home --o--> Explorer --Esc--> Home
/// Home --o--> Explorer --Enter--> Viewer --b--> Explorer --Esc--> Home
/// ```
///
/// # Assertions
///
/// - Explorer `Esc` always navigates to Home.
/// - Viewer `b` always navigates to Explorer.
/// - Returning through explicit destinations recreates Explorer at its root.
#[test]
fn explicit_destinations_navigate_between_pages() -> leptatui::Result<()> {
    let tree = TestTree::new("explicit-destinations");
    fs::write(tree.root().join("guide.md"), "# Guide")
        .expect("the Markdown file should be created");
    let contexts = TestContexts::new(tree.root());
    let mut view = contexts.view();
    let mut terminal = Terminal::new(TestBackend::new(80, 18))?;
    draw_editor(&mut terminal, &view)?;

    for key in [KeyCode::Char('o'), KeyCode::Esc] {
        assert_eq!(
            view.handle_key_event(KeyEvent::new(key, KeyModifiers::NONE))?,
            KeyControl::Handled
        );
        draw_editor(&mut terminal, &view)?;
    }
    assert!(
        rendered_lines(&terminal)
            .join("\n")
            .contains("Recent files")
    );

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_editor(&mut terminal, &view)?;
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_editor(&mut terminal, &view)?;
    assert!(
        rendered_lines(&terminal)
            .join("\n")
            .contains("Markdown viewer")
    );

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_editor(&mut terminal, &view)?;
    assert!(
        rendered_lines(&terminal)
            .join("\n")
            .contains("> [M] guide.md")
    );
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_editor(&mut terminal, &view)?;
    assert!(
        rendered_lines(&terminal)
            .join("\n")
            .contains("Recent files")
    );

    Ok(())
}

/// Verifies Explorer owns selection and directory state per route instance.
///
/// # Example Under Test
///
/// ```text
/// Explorer: enter docs/
/// Home
/// Explorer
/// ```
///
/// # Assertions
///
/// - Entering `docs` displays its nested Markdown file.
/// - Leaving and returning displays the workspace root.
/// - The root's first entry receives a fresh selection.
#[test]
fn explorer_resets_to_root_after_leaving_the_route() -> leptatui::Result<()> {
    let tree = TestTree::new("explorer-route-reset");
    let docs = tree.root().join("docs");
    fs::create_dir(&docs).expect("the docs directory should be created");
    fs::write(docs.join("guide.md"), "# Guide")
        .expect("the nested Markdown file should be created");
    let contexts = TestContexts::new(tree.root());
    let mut view = contexts.view();
    let mut terminal = Terminal::new(TestBackend::new(80, 18))?;
    draw_editor(&mut terminal, &view)?;

    for key in [KeyCode::Char('o'), KeyCode::Enter] {
        assert_eq!(
            view.handle_key_event(KeyEvent::new(key, KeyModifiers::NONE))?,
            KeyControl::Handled
        );
        draw_editor(&mut terminal, &view)?;
    }
    let nested = rendered_lines(&terminal).join("\n");
    assert!(nested.contains("Directory: docs"));
    assert!(nested.contains("> [M] guide.md"));

    for key in [KeyCode::Esc, KeyCode::Char('o')] {
        assert_eq!(
            view.handle_key_event(KeyEvent::new(key, KeyModifiers::NONE))?,
            KeyControl::Handled
        );
        draw_editor(&mut terminal, &view)?;
    }
    let returned = rendered_lines(&terminal).join("\n");
    assert!(returned.contains("Directory: ."));
    assert!(returned.contains("> [D] docs"));

    Ok(())
}

/// Verifies page-local Explorer selection clamps at listing boundaries.
///
/// # Example Under Test
///
/// ```text
/// alpha.md
/// beta.md
/// Up, Down, Down
/// ```
///
/// # Assertions
///
/// - Moving above the first entry retains `alpha.md`.
/// - Moving below the final entry retains `beta.md`.
#[test]
fn explorer_selection_clamps_at_listing_boundaries() -> leptatui::Result<()> {
    let tree = TestTree::new("explorer-selection-boundaries");
    fs::write(tree.root().join("alpha.md"), "# Alpha")
        .expect("the first Markdown file should be created");
    fs::write(tree.root().join("beta.md"), "# Beta")
        .expect("the second Markdown file should be created");
    let contexts = TestContexts::new(tree.root());
    let mut view = contexts.view();
    let mut terminal = Terminal::new(TestBackend::new(80, 18))?;
    draw_editor(&mut terminal, &view)?;
    view.handle_key_event(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE))?;
    draw_editor(&mut terminal, &view)?;

    view.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))?;
    draw_editor(&mut terminal, &view)?;
    assert!(
        rendered_lines(&terminal)
            .join("\n")
            .contains("> [M] alpha.md")
    );

    for _ in 0..2 {
        view.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))?;
    }
    draw_editor(&mut terminal, &view)?;
    assert!(
        rendered_lines(&terminal)
            .join("\n")
            .contains("> [M] beta.md")
    );

    Ok(())
}
