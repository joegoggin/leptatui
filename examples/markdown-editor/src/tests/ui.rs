//! Routed keyboard interaction and rendering tests.

use std::{
    cell::{Cell, RefCell},
    fs,
    rc::Rc,
};

use leptatui::prelude::{KeyCode, KeyControl, KeyEvent, KeyModifiers, View};
use ratatui::{Terminal, backend::TestBackend};

use crate::{
    controller::Controller, editor_process::EditorProcess, filesystem::FileSystem, ui::app_view,
};

use super::support::{TestTree, draw_editor, rendered_lines};

/// Verifies Home, Explorer, and Viewer form one routed file-opening workflow.
///
/// # Example Under Test
///
/// ```text
/// Home
/// o
/// Explorer: Down, Enter
/// Viewer: PageDown, r, h
/// ```
///
/// # Assertions
///
/// - The application starts on Home with an empty recent list.
/// - `o` opens Explorer and selection remains interactive.
/// - Activating a Markdown file opens Viewer.
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
    let controller = Rc::new(RefCell::new(
        Controller::initialize(tree.root(), FileSystem::new(), EditorProcess::new())
            .expect("the workspace should initialize"),
    ));
    let mut view = app_view(controller, Rc::new(Cell::new(false)));
    let mut terminal = Terminal::new(TestBackend::new(80, 18))?;

    draw_editor(&mut terminal, &view)?;
    let home = rendered_lines(&terminal).join("\n");
    assert!(home.contains("Markdown editor"));
    assert!(home.contains("Open file"));
    assert!(home.contains("No recent Markdown files"));

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_editor(&mut terminal, &view)?;
    assert!(
        rendered_lines(&terminal)
            .join("\n")
            .contains("> [M] alpha.md")
    );

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?,
        KeyControl::Handled
    );
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

    Ok(())
}

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
    let controller = Rc::new(RefCell::new(
        Controller::initialize(tree.root(), FileSystem::new(), EditorProcess::new())
            .expect("the workspace should initialize"),
    ));
    let edit_requested = Rc::new(Cell::new(false));
    let mut view = app_view(Rc::clone(&controller), Rc::clone(&edit_requested));
    let mut terminal = Terminal::new(TestBackend::new(80, 18))?;
    draw_editor(&mut terminal, &view)?;

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE))?,
        KeyControl::Pass
    );
    assert!(!edit_requested.get());

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
    assert!(edit_requested.get());

    let quit_requested = Rc::new(Cell::new(false));
    let mut quit_view = app_view(controller, Rc::clone(&quit_requested));
    assert_eq!(
        quit_view.handle_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))?,
        KeyControl::Exit
    );
    assert!(!quit_requested.get());

    Ok(())
}

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
    let controller = Rc::new(RefCell::new(
        Controller::initialize(tree.root(), FileSystem::new(), EditorProcess::new())
            .expect("the workspace should initialize"),
    ));
    let mut view = app_view(controller, Rc::new(Cell::new(false)));
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
    let controller = Rc::new(RefCell::new(
        Controller::initialize(tree.root(), FileSystem::new(), EditorProcess::new())
            .expect("the workspace should initialize"),
    ));
    let mut view = app_view(controller, Rc::new(Cell::new(false)));
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
    let controller = Rc::new(RefCell::new(
        Controller::initialize(tree.root(), FileSystem::new(), EditorProcess::new())
            .expect("the workspace should initialize"),
    ));
    let mut view = app_view(controller, Rc::new(Cell::new(false)));
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
