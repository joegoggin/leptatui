//! Cross-page routing and responsive-layout tests.

use super::*;

/// Verifies every routed page remains usable in a narrow terminal.
///
/// # Example Under Test
///
/// ```text
/// viewport = 50x20
/// Home --o--> Explorer --Enter--> Viewer
/// ```
///
/// # Assertions
///
/// - Home renders its open action.
/// - Explorer renders its selected Markdown row.
/// - Viewer renders the selected Markdown content.
#[test]
fn routed_pages_render_in_a_narrow_terminal() -> leptatui::Result<()> {
    let tree = TestTree::new("narrow-routes");
    fs::write(tree.root().join("guide.md"), "# Narrow guide")
        .expect("the Markdown file should be created");
    let controller = Rc::new(RefCell::new(
        Controller::initialize(tree.root(), FileSystem::new(), EditorProcess::new())
            .expect("the workspace should initialize"),
    ));
    let mut view = app_view(controller, Rc::new(Cell::new(false)));
    let mut terminal = Terminal::new(TestBackend::new(50, 20))?;

    draw_editor(&mut terminal, &view)?;
    assert!(rendered_lines(&terminal).join("\n").contains("Open file"));

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_editor(&mut terminal, &view)?;
    assert!(
        rendered_lines(&terminal)
            .join("\n")
            .contains("> [M] guide.md")
    );

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_editor(&mut terminal, &view)?;
    assert!(
        rendered_lines(&terminal)
            .join("\n")
            .contains("Narrow guide")
    );

    Ok(())
}
