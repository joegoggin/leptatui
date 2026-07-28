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
/// - Returning through explicit destinations preserves the current explorer.
#[test]
fn explicit_destinations_navigate_between_pages() -> leptatui::Result<()> {
    let tree = TestTree::new("explicit-destinations");
    fs::write(tree.root().join("guide.md"), "# Guide")
        .expect("the Markdown file should be created");
    let controller = Rc::new(RefCell::new(
        Controller::initialize(tree.root(), FileSystem::new(), EditorProcess::new())
            .expect("the workspace should initialize"),
    ));
    let mut view = app_view(controller, Rc::new(Cell::new(false)));
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
