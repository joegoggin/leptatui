//! Viewer page interaction and rendering tests.

use super::*;

/// Verifies editing is available only from Viewer and preserves global quit.
///
/// # Example Under Test
///
/// ```text
/// Home: e
/// Home: o
/// Explorer: Enter
/// Viewer: e
/// Home: q
/// ```
///
/// # Assertions
///
/// - `e` passes without an open Viewer document.
/// - Explorer activation opens the selected Markdown document.
/// - Viewer `e` requests a restored-terminal edit and exits the session.
/// - Global `q` exits without setting an edit request.
#[test]
fn viewer_edit_key_requests_an_external_session_only_for_an_open_document() -> leptatui::Result<()>
{
    let tree = TestTree::new("viewer-edit-key");
    fs::write(tree.root().join("guide.md"), "# Guide")
        .expect("the Markdown file should be created");
    let contexts = TestContexts::new(tree.root());
    let mut view = contexts.view();
    let mut terminal = Terminal::new(TestBackend::new(80, 18))?;
    draw_editor(&mut terminal, &view)?;

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE))?,
        KeyControl::Pass
    );
    assert_eq!(contexts.files.edit_request.get_untracked(), None);

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
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE))?,
        KeyControl::Exit
    );
    assert!(contexts.files.edit_request.get_untracked().is_some());

    contexts.files.edit_request.set(None);
    let mut quit_view = contexts.view();
    assert_eq!(
        quit_view.handle_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))?,
        KeyControl::Exit
    );
    assert_eq!(contexts.files.edit_request.get_untracked(), None);

    Ok(())
}

/// Verifies the shared Markdown view owns file failures and reload recovery.
///
/// # Example Under Test
///
/// ```text
/// open guide.md
/// delete guide.md
/// r
/// recreate guide.md
/// r
/// ```
///
/// # Assertions
///
/// - The initial document renders successfully.
/// - Reloading after deletion renders the shared Markdown file diagnostic.
/// - Reloading after recreation renders the restored document.
#[test]
fn viewer_markdown_view_recovers_after_a_missing_file_returns() -> leptatui::Result<()> {
    let tree = TestTree::new("viewer-reload-recovery");
    let guide = tree.root().join("guide.md");
    fs::write(&guide, "# Original").expect("the Markdown file should be created");
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
    assert!(rendered_lines(&terminal).join("\n").contains("Original"));

    fs::remove_file(&guide).expect("the open Markdown file should be removed");
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_editor(&mut terminal, &view)?;
    assert!(
        rendered_lines(&terminal)
            .join("\n")
            .contains("failed to read Markdown file")
    );

    fs::write(&guide, "# Restored").expect("the Markdown file should be recreated");
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_editor(&mut terminal, &view)?;
    assert!(rendered_lines(&terminal).join("\n").contains("Restored"));

    Ok(())
}

/// Verifies invalid UTF-8 diagnostics come from the shared Markdown view.
///
/// # Example Under Test
///
/// ```text
/// workspace/
/// └── invalid.md = [0xff, 0xfe, 0xfd]
/// ```
///
/// # Assertions
///
/// - Explorer opens the path in Viewer.
/// - Viewer renders the shared Markdown file diagnostic.
/// - The diagnostic identifies the path and invalid UTF-8 content.
#[test]
fn viewer_markdown_view_renders_invalid_utf8_diagnostics() -> leptatui::Result<()> {
    let tree = TestTree::new("viewer-invalid-utf8");
    let invalid = tree.root().join("invalid.md");
    fs::write(&invalid, [0xff, 0xfe, 0xfd]).expect("the invalid UTF-8 fixture should be created");
    let contexts = TestContexts::new(tree.root());
    let mut view = contexts.view();
    let mut terminal = Terminal::new(TestBackend::new(100, 18))?;

    draw_editor(&mut terminal, &view)?;
    for key in [KeyCode::Char('o'), KeyCode::Enter] {
        assert_eq!(
            view.handle_key_event(KeyEvent::new(key, KeyModifiers::NONE))?,
            KeyControl::Handled
        );
        draw_editor(&mut terminal, &view)?;
    }
    let rendered = rendered_lines(&terminal).join("\n");
    assert!(rendered.contains("failed to read Markdown file"));
    assert!(rendered.contains("invalid.md"));
    assert!(rendered.to_lowercase().contains("utf-8"));

    Ok(())
}

/// Verifies Viewer inherits local navigation from the shared Markdown view.
///
/// # Example Under Test
///
/// ```text
/// guide.md = "[Next](next.md)"
/// next.md = "# Linked document"
/// Enter focused link
/// Shift+H
/// ```
///
/// # Assertions
///
/// - The root Markdown document renders its local link.
/// - Activating the focused link renders the linked document in place.
/// - Markdown back history restores the root document.
#[test]
fn viewer_markdown_view_navigates_local_links_and_history() -> leptatui::Result<()> {
    let tree = TestTree::new("viewer-markdown-navigation");
    fs::write(tree.root().join("guide.md"), "# Guide\n\n[Next](next.md)")
        .expect("the root Markdown file should be created");
    fs::write(tree.root().join("next.md"), "# Linked document")
        .expect("the linked Markdown file should be created");
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
    assert!(rendered_lines(&terminal).join("\n").contains("Next"));

    for key in [KeyCode::Tab, KeyCode::Enter] {
        assert_eq!(
            view.handle_key_event(KeyEvent::new(key, KeyModifiers::NONE))?,
            KeyControl::Handled
        );
        draw_editor(&mut terminal, &view)?;
    }
    assert!(
        rendered_lines(&terminal)
            .join("\n")
            .contains("Linked document")
    );

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::SHIFT,))?,
        KeyControl::Handled
    );
    draw_editor(&mut terminal, &view)?;
    assert!(rendered_lines(&terminal).join("\n").contains("Guide"));

    Ok(())
}
