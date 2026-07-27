//! End-to-end non-interactive Markdown editor workflow coverage.

use std::{
    cell::{Cell, RefCell},
    ffi::OsString,
    fs,
    rc::Rc,
};

use leptatui::prelude::{KeyCode, KeyControl, KeyEvent, KeyModifiers, View};
use ratatui::{Terminal, backend::TestBackend};

use crate::{
    controller::Controller, editor_process::EditorProcess, filesystem::FileSystem, ui::app_view,
};

use super::support::{
    RecordingLauncher, TestEnvironment, TestLaunchOutcome, TestTree, draw_editor, rendered_lines,
};

/// Verifies the explorer, preview, editor, and renderer complete one workflow.
///
/// # Example Under Test
///
/// ```text
/// workspace/
/// └── docs/
///     └── guide.md = "# Before edit"
///
/// Enter docs
/// Enter guide.md
/// e
/// configured-editor -- /absolute/workspace/docs/guide.md
/// ```
///
/// # Assertions
///
/// - UI key events enter the nested directory and open its Markdown file.
/// - Test-backend rendering exposes the current path and original document.
/// - The edit key exits the TUI session and records an edit request.
/// - The injected editor receives the canonical path and replaces the source.
/// - A rebuilt view renders the edited source with explorer context preserved.
///
/// # Why
///
/// This covers the reference application's cross-layer contract without
/// requiring an interactive terminal or spawning a real editor.
#[test]
fn workflow_browses_previews_edits_and_renders_without_a_terminal() -> leptatui::Result<()> {
    let tree = TestTree::new("end-to-end-workflow");
    let docs = tree.root().join("docs");
    let guide = docs.join("guide.md");
    fs::create_dir(&docs).expect("the docs directory should be created");
    fs::write(&guide, "# Before edit").expect("the Markdown file should be created");
    let canonical_docs = fs::canonicalize(&docs).expect("the docs directory should canonicalize");
    let canonical_guide = fs::canonicalize(&guide).expect("the guide should canonicalize");
    let commands = Rc::new(RefCell::new(Vec::new()));
    let editor_process = EditorProcess::with_services(
        RecordingLauncher {
            commands: Rc::clone(&commands),
            outcome: TestLaunchOutcome::Success,
            replacement: Some((canonical_guide.clone(), String::from("# After edit"))),
        },
        TestEnvironment {
            visual: Some(OsString::from("configured-editor")),
            editor: None,
        },
    );
    let controller = Rc::new(RefCell::new(
        Controller::initialize(tree.root(), FileSystem::new(), editor_process)
            .expect("the workspace should initialize"),
    ));
    let edit_requested = Rc::new(Cell::new(false));
    let mut view = app_view(Rc::clone(&controller), Rc::clone(&edit_requested));
    let mut terminal = Terminal::new(TestBackend::new(90, 24))?;

    draw_editor(&mut terminal, &view)?;
    assert!(rendered_lines(&terminal).join("\n").contains("> [D] docs"));

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    assert_eq!(controller.borrow().explorer().directory(), canonical_docs);
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_editor(&mut terminal, &view)?;
    let before_edit = rendered_lines(&terminal).join("\n");
    assert!(before_edit.contains("Directory: docs"));
    assert!(before_edit.contains("Open: docs/guide.md"));
    assert!(before_edit.contains("Before edit"));

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE))?,
        KeyControl::Exit
    );
    assert!(edit_requested.get());
    drop(view);

    let expected_selection = controller.borrow().explorer().selection();
    assert!(controller.borrow_mut().edit_preview());
    assert_eq!(
        commands.borrow().as_slice(),
        [(
            OsString::from("configured-editor"),
            vec![
                OsString::from("--"),
                canonical_guide.clone().into_os_string(),
            ],
        )]
    );
    assert_eq!(controller.borrow().explorer().directory(), canonical_docs);
    assert_eq!(
        controller.borrow().explorer().selection(),
        expected_selection
    );
    assert_eq!(controller.borrow().preview().source(), Some("# After edit"));

    let rebuilt_view = app_view(Rc::clone(&controller), Rc::new(Cell::new(false)));
    draw_editor(&mut terminal, &rebuilt_view)?;
    let after_edit = rendered_lines(&terminal).join("\n");
    assert!(after_edit.contains("Directory: docs"));
    assert!(after_edit.contains("Open: docs/guide.md"));
    assert!(after_edit.contains("After edit"));

    Ok(())
}
