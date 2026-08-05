//! Home page workflow and recent-file tests.

use super::*;

/// Verifies Viewer reload and Home recent-file behavior remain integrated.
///
/// # Example Under Test
///
/// ```text
/// Viewer: PageDown, r, h
/// ```
///
/// # Assertions
///
/// - Viewer opens the requested Markdown file.
/// - Viewer scrolling and reload retain their existing behavior.
/// - Returning Home exposes the opened file in recent history.
#[test]
fn routed_pages_open_reload_and_remember_a_markdown_file() -> leptatui::Result<()> {
    let tree = TestTree::new("routed-pages");
    fs::write(tree.root().join("alpha.md"), "# Alpha")
        .expect("the first Markdown file should be created");
    let beta_path = tree.root().join("beta.md");
    let beta_source = (0..24)
        .map(|index| format!("## Beta line {index}\n"))
        .collect::<String>();
    fs::write(&beta_path, beta_source).expect("the long Markdown file should be created");
    let contexts = TestContexts::new(tree.root());
    let mut view = contexts.view_at(crate::pages::viewer_location(&beta_path));
    let mut terminal = Terminal::new(TestBackend::new(80, 18))?;

    draw_editor(&mut terminal, &view)?;
    let before_scroll = rendered_lines(&terminal);
    assert!(before_scroll.join("\n").contains("Markdown viewer"));
    assert!(before_scroll.join("\n").contains("Beta line 0"));

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_editor(&mut terminal, &view)?;
    let after_scroll = rendered_lines(&terminal);
    assert_ne!(after_scroll, before_scroll);
    assert!(!after_scroll.join("\n").contains("Beta line 0"));

    fs::write(&beta_path, "# Reloaded Beta")
        .expect("the Markdown file should be updated before reload");
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_editor(&mut terminal, &view)?;
    assert!(
        rendered_lines(&terminal)
            .join("\n")
            .contains("Reloaded Beta")
    );

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_editor(&mut terminal, &view)?;
    let returned_home = rendered_lines(&terminal).join("\n");
    assert!(returned_home.contains("Recent files"));
    assert!(returned_home.contains("beta.md"));

    for key in [KeyCode::Tab, KeyCode::Tab] {
        assert_eq!(
            view.handle_key_event(KeyEvent::new(key, KeyModifiers::NONE))?,
            KeyControl::Handled
        );
        draw_editor(&mut terminal, &view)?;
    }
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

    Ok(())
}
