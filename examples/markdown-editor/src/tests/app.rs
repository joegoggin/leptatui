//! Application routing and responsive-layout tests.

use std::{fs, thread, time::Duration};

use leptatui::prelude::{AppRoot, KeyCode, KeyControl, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};

use crate::pages::viewer_location;

use super::support::{TestContexts, TestTree, draw_editor, rendered_lines};

/// Verifies the routed Viewer remains usable in a narrow terminal.
///
/// # Example Under Test
///
/// ```text
/// viewport = 50x20
/// /view?path=guide.md
/// ```
///
/// # Assertions
///
/// - Viewer renders the selected Markdown content at 50 columns.
#[test]
fn routed_pages_render_in_a_narrow_terminal() -> leptatui::Result<()> {
    let tree = TestTree::new("narrow-routes");
    fs::write(tree.root().join("guide.md"), "# Narrow guide")
        .expect("the Markdown file should be created");
    let contexts = TestContexts::new(tree.root());
    let view = contexts.view_at(viewer_location(&tree.root().join("guide.md")));
    let mut terminal = Terminal::new(TestBackend::new(50, 20))?;

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
/// Viewer --h--> Home
/// AppRoot render
/// ```
///
/// # Assertions
///
/// - The managed root renders Home successfully.
/// - The Viewer shortcut handles navigation.
/// - The managed root renders Home after navigation.
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
    let mut view = contexts.view_at(viewer_location(&tree.root().join("guide.md")));
    let mut terminal = Terminal::new(TestBackend::new(50, 20))?;
    let mut render_result = Ok(());

    terminal.draw(|frame| {
        render_result = AppRoot::render(&mut view, frame);
    })?;
    render_result?;

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );

    for _ in 0..3 {
        thread::sleep(Duration::from_millis(20));
        render_result = Ok(());
        terminal.draw(|frame| {
            render_result = AppRoot::render(&mut view, frame);
        })?;
        render_result?;
    }

    assert!(
        rendered_lines(&terminal)
            .join("\n")
            .contains("Markdown editor")
    );

    Ok(())
}
