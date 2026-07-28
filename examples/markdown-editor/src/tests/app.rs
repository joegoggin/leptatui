//! Application routing and responsive-layout tests.

use std::fs;

use leptatui::prelude::{AppRoot, KeyCode, KeyControl, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};

use super::support::{TestContexts, TestTree, draw_editor, rendered_lines};

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
    let contexts = TestContexts::new(tree.root());
    let mut view = contexts.view();
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

/// Verifies managed rendering preserves shared context after route navigation.
///
/// # Example Under Test
///
/// ```text
/// AppRoot render
/// Home --o--> Explorer
/// AppRoot render
/// ```
///
/// # Assertions
///
/// - The managed root renders Home successfully.
/// - The Home shortcut handles navigation.
/// - The managed root renders the selected Markdown entry in Explorer.
///
/// # Why
///
/// The managed root creates a temporary render scope, so context provided by a
/// lazily constructed application component must persist in its owner.
#[test]
fn managed_root_preserves_context_after_route_navigation() -> leptatui::Result<()> {
    let tree = TestTree::new("managed-context");
    fs::write(tree.root().join("guide.md"), "# Managed guide")
        .expect("the Markdown file should be created");
    let contexts = TestContexts::new(tree.root());
    let mut view = contexts.view();
    let mut terminal = Terminal::new(TestBackend::new(50, 20))?;
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        render_result = AppRoot::render(&mut view, frame);
    })?;
    render_result?;

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    render_result = Ok(());
    terminal.draw(|frame| {
        render_result = AppRoot::render(&mut view, frame);
    })?;
    render_result?;

    assert!(
        rendered_lines(&terminal)
            .join("\n")
            .contains("> [M] guide.md")
    );

    Ok(())
}
