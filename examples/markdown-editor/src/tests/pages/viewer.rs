//! Viewer page interaction and rendering tests.

use super::*;

/// Verifies editing is available only from Viewer and propagates failures.
///
/// # Example Under Test
///
/// ```text
/// Home: e
/// direct Viewer route
/// Viewer: e
/// error screen: q
/// ```
///
/// # Assertions
///
/// - `e` passes without an open Viewer document.
/// - A direct Viewer route opens the selected Markdown document.
/// - Viewer `e` requests an external edit without exiting the session.
/// - A headless editor launch failure renders the standard error screen.
/// - Global `q` still exits the mounted application.
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
    view = contexts.view_at(crate::pages::viewer_location(&tree.root().join("guide.md")));
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_editor(&mut terminal, &view)?;
    let rendered = rendered_lines(&terminal).join("\n");
    assert!(rendered.contains("Error"));
    assert!(rendered.contains("external editing requires a managed Leptatui application"));

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))?,
        KeyControl::Exit
    );

    Ok(())
}

/// Verifies Viewer route validation uses the standard error screen.
///
/// # Example Under Test
///
/// ```text
/// /view?path=notes.txt
/// ```
///
/// # Assertions
///
/// - The Viewer renders the standard error screen.
/// - The diagnostic identifies the unsupported Markdown path.
#[test]
fn viewer_non_markdown_path_renders_standard_error_screen() -> leptatui::Result<()> {
    let tree = TestTree::new("viewer-non-markdown");
    let notes = tree.root().join("notes.txt");
    fs::write(&notes, "Notes").expect("the non-Markdown file should be created");
    let contexts = TestContexts::new(tree.root());
    let view = contexts.view_at(crate::pages::viewer_location(&notes));
    let mut terminal = Terminal::new(TestBackend::new(100, 18))?;

    draw_editor(&mut terminal, &view)?;
    let rendered = rendered_lines(&terminal).join("\n");
    assert!(rendered.contains("Error"));
    assert!(rendered.contains("preview path is not a Markdown file"));
    assert!(rendered.contains("notes.txt"));

    Ok(())
}

/// Verifies asynchronous Markdown failures use the standard error screen.
///
/// # Example Under Test
///
/// ```text
/// open guide.md
/// delete guide.md
/// r
/// ```
///
/// # Assertions
///
/// - The Markdown fixture is created and removed successfully.
/// - The initial document renders successfully.
/// - Reload is handled after the source is deleted.
/// - Reloading after deletion renders a standard error screen naming the path.
#[test]
fn viewer_markdown_view_throws_view_error_after_reload_failure() -> leptatui::Result<()> {
    let tree = TestTree::new("viewer-reload-recovery");
    let guide = tree.root().join("guide.md");
    fs::write(&guide, "# Original").expect("the Markdown file should be created");
    let contexts = TestContexts::new(tree.root());
    let mut view = contexts.view_at(crate::pages::viewer_location(&guide));
    let mut terminal = Terminal::new(TestBackend::new(80, 18))?;

    draw_editor(&mut terminal, &view)?;
    assert!(rendered_lines(&terminal).join("\n").contains("Original"));

    fs::remove_file(&guide).expect("the open Markdown file should be removed");
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_editor(&mut terminal, &view)?;
    let rendered = rendered_lines(&terminal).join("\n");
    assert!(rendered.contains("Error"));
    assert!(rendered.contains("guide.md"));

    Ok(())
}

/// Verifies invalid UTF-8 produces the standard ViewError screen.
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
/// - A direct Viewer route opens the path.
/// - Viewer renders the standard error screen.
/// - The diagnostic identifies the path and invalid UTF-8 content.
#[test]
fn viewer_markdown_view_renders_invalid_utf8_diagnostics() -> leptatui::Result<()> {
    let tree = TestTree::new("viewer-invalid-utf8");
    let invalid = tree.root().join("invalid.md");
    fs::write(&invalid, [0xff, 0xfe, 0xfd]).expect("the invalid UTF-8 fixture should be created");
    let contexts = TestContexts::new(tree.root());
    let view = contexts.view_at(crate::pages::viewer_location(&invalid));
    let mut terminal = Terminal::new(TestBackend::new(100, 18))?;

    draw_editor(&mut terminal, &view)?;
    let rendered = rendered_lines(&terminal).join("\n");
    assert!(rendered.contains("Error"));
    assert!(rendered.contains("invalid.md"));
    assert!(rendered.to_lowercase().contains("utf-8"));

    Ok(())
}

/// Verifies Viewer disables line-number gutters on Markdown code blocks.
///
/// # Example Under Test
///
/// ```text
/// guide.md = fenced text block containing alpha and beta lines
/// ```
///
/// # Assertions
///
/// - The Markdown fixture is created successfully.
/// - The source content renders in Viewer.
/// - The rendered code block omits its line-number gutter.
#[test]
fn viewer_markdown_view_disables_code_line_numbers() -> leptatui::Result<()> {
    let tree = TestTree::new("viewer-no-line-numbers");
    let guide = tree.root().join("guide.md");
    fs::write(&guide, "```text\nalpha\nbeta\n```\n").expect("the Markdown file should be created");
    let contexts = TestContexts::new(tree.root());
    let view = contexts.view_at(crate::pages::viewer_location(&guide));
    let mut terminal = Terminal::new(TestBackend::new(80, 18))?;

    draw_editor(&mut terminal, &view)?;
    let rendered = rendered_lines(&terminal).join("\n");
    assert!(rendered.contains("alpha"));
    assert!(!rendered.contains("1 │ alpha"));

    Ok(())
}

/// Verifies scrolling Markdown preserves the Viewer document borders.
///
/// # Example Under Test
///
/// ```text
/// guide.md = 40 Markdown headings
/// terminal = 80x18
/// PageDown
/// ```
///
/// # Assertions
///
/// - Page Down scrolls the overflowing Markdown document.
/// - The document's top border remains intact.
/// - The document's bottom border remains intact.
///
/// # Why
///
/// The Markdown document owns the scroll range while the surrounding block
/// owns the border and must clip scrolling descendants to its content area.
#[test]
fn viewer_scrolling_preserves_document_borders() -> leptatui::Result<()> {
    let tree = TestTree::new("viewer-scroll-borders");
    let source = (0..40)
        .map(|index| format!("## Guide section {index}\n"))
        .collect::<String>();
    fs::write(tree.root().join("guide.md"), source)
        .expect("the long Markdown file should be created");
    let contexts = TestContexts::new(tree.root());
    let mut view = contexts.view_at("/view?path=guide.md");
    let mut terminal = Terminal::new(TestBackend::new(80, 18))?;

    draw_editor(&mut terminal, &view)?;
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_editor(&mut terminal, &view)?;

    let rendered = rendered_lines(&terminal);
    let top_border = rendered[4].chars().collect::<Vec<_>>();
    let bottom_border = rendered[11].chars().collect::<Vec<_>>();
    assert_eq!(top_border[2], '┌');
    assert!(top_border[3..77].iter().all(|symbol| *symbol == '─'));
    assert_eq!(top_border[77], '┐');
    assert_eq!(bottom_border[2], '└');
    assert!(bottom_border[3..77].iter().all(|symbol| *symbol == '─'));
    assert_eq!(bottom_border[77], '┘');

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
    let mut view = contexts.view_at(crate::pages::viewer_location(&tree.root().join("guide.md")));
    let mut terminal = Terminal::new(TestBackend::new(80, 18))?;

    draw_editor(&mut terminal, &view)?;
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
