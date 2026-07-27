//! Keyboard interaction and responsive rendering tests.

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

use super::support::{TestTree, draw_editor, rendered_lines, rendered_position};

/// Verifies interactive keys update selection, preview, reload, and scrolling.
///
/// # Example Under Test
///
/// ```text
/// Down
/// Enter
/// PageDown
/// r
/// ```
///
/// # Assertions
///
/// - The initial selected row is `alpha.md`.
/// - `Down` moves the selected marker to `beta.md`.
/// - `Enter` opens and renders `beta.md`.
/// - `PageDown` scrolls the preview even when the explorer also overflows.
/// - `r` reloads and renders changed Markdown without closing the preview.
///
/// # Why
///
/// Keyboard scrolling should target the Markdown preview before the explorer.
#[test]
fn editor_keys_drive_selection_preview_reload_and_scroll() -> leptatui::Result<()> {
    let tree = TestTree::new("editor-keys");
    fs::write(tree.root().join("alpha.md"), "# Alpha")
        .expect("the first Markdown file should be created");
    let beta_source = (0..24)
        .map(|index| format!("## Beta line {index}\n"))
        .collect::<String>();
    let beta_path = tree.root().join("beta.md");
    fs::write(&beta_path, beta_source).expect("the long Markdown file should be created");
    for index in 0..24 {
        fs::write(
            tree.root().join(format!("extra-{index:02}.md")),
            format!("# Extra {index}"),
        )
        .expect("each extra Markdown file should be created");
    }
    let controller = Rc::new(RefCell::new(
        Controller::initialize(tree.root(), FileSystem::new(), EditorProcess::new())
            .expect("the workspace should initialize"),
    ));
    let mut view = app_view(controller, Rc::new(Cell::new(false)));
    let mut terminal = Terminal::new(TestBackend::new(80, 18))?;

    draw_editor(&mut terminal, &view)?;
    let initial_render = rendered_lines(&terminal).join("\n");
    assert!(initial_render.contains("> [M] alpha.md"));
    assert!(initial_render.contains("e edit"));
    assert!(initial_render.contains("q quit"));

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_editor(&mut terminal, &view)?;
    assert!(
        rendered_lines(&terminal)
            .join("\n")
            .contains("> [M] beta.md")
    );

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_editor(&mut terminal, &view)?;
    let before_scroll = rendered_lines(&terminal);
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
    let reloaded = rendered_lines(&terminal).join("\n");
    assert!(reloaded.contains("Open: "));
    assert!(reloaded.contains("Reloaded Beta"));

    Ok(())
}

/// Verifies the edit key exits only to edit an open preview.
///
/// # Example Under Test
///
/// ```text
/// e
/// Enter
/// e
/// ```
///
/// # Assertions
///
/// - `e` is handled without exiting when no preview is open.
/// - `Enter` opens the selected Markdown document.
/// - `e` then requests editing and exits the managed TUI loop.
/// - `q` exits without setting a separate edit request.
#[test]
fn editor_edit_key_requests_an_external_session_only_for_an_open_preview() -> leptatui::Result<()> {
    let tree = TestTree::new("editor-edit-key");
    fs::write(tree.root().join("guide.md"), "# Guide")
        .expect("the Markdown file should be created");
    let controller = Rc::new(RefCell::new(
        Controller::initialize(tree.root(), FileSystem::new(), EditorProcess::new())
            .expect("the workspace should initialize"),
    ));
    let edit_requested = Rc::new(Cell::new(false));
    let mut view = app_view(Rc::clone(&controller), Rc::clone(&edit_requested));

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    assert!(!edit_requested.get());
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?,
        KeyControl::Handled
    );
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

/// Verifies the workspace switches from side-by-side to stacked panes.
///
/// # Example Under Test
///
/// ```text
/// viewport = 100x30
/// viewport = 50x30
/// ```
///
/// # Assertions
///
/// - Wide rendering places Explorer and Preview headings on the same row.
/// - Narrow rendering places Preview below Explorer.
/// - Both layouts render the selected Markdown document.
#[test]
fn editor_renders_wide_and_narrow_pane_layouts() -> leptatui::Result<()> {
    let tree = TestTree::new("responsive-layout");
    fs::write(tree.root().join("guide.md"), "# Responsive guide")
        .expect("the Markdown file should be created");

    let mut wide_controller =
        Controller::initialize(tree.root(), FileSystem::new(), EditorProcess::new())
            .expect("the wide controller should initialize");
    assert!(wide_controller.activate_selected());
    let wide_view = app_view(
        Rc::new(RefCell::new(wide_controller)),
        Rc::new(Cell::new(false)),
    );
    let mut wide_terminal = Terminal::new(TestBackend::new(100, 30))?;
    draw_editor(&mut wide_terminal, &wide_view)?;
    let (_, wide_explorer_row) =
        rendered_position(&wide_terminal, "Explorer").expect("Explorer should render");
    let (_, wide_preview_row) =
        rendered_position(&wide_terminal, "Preview").expect("Preview should render");
    assert_eq!(wide_explorer_row, wide_preview_row);
    assert!(
        rendered_lines(&wide_terminal)
            .join("\n")
            .contains("Responsive guide")
    );

    let mut narrow_controller =
        Controller::initialize(tree.root(), FileSystem::new(), EditorProcess::new())
            .expect("the narrow controller should initialize");
    assert!(narrow_controller.activate_selected());
    let narrow_view = app_view(
        Rc::new(RefCell::new(narrow_controller)),
        Rc::new(Cell::new(false)),
    );
    let mut narrow_terminal = Terminal::new(TestBackend::new(50, 30))?;
    draw_editor(&mut narrow_terminal, &narrow_view)?;
    let (_, narrow_explorer_row) =
        rendered_position(&narrow_terminal, "Explorer").expect("Explorer should render");
    let (_, narrow_preview_row) =
        rendered_position(&narrow_terminal, "Preview").expect("Preview should render");
    assert!(narrow_preview_row > narrow_explorer_row);
    let narrow_text = rendered_lines(&narrow_terminal).join("\n");
    assert!(
        narrow_text.contains("Responsive guide"),
        "narrow rendering: {narrow_text:?}"
    );

    Ok(())
}
