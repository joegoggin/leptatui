//! End-to-end non-interactive Markdown editor workflow coverage.

use std::{
    ffi::OsString,
    fs,
    sync::{Arc, Mutex},
};

use leptatui::prelude::{KeyCode, KeyControl, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};

use crate::{pages::viewer_location, services::EditorProcess};

use super::support::{
    RecordingLauncher, TestContexts, TestEnvironment, TestLaunchOutcome, TestTree, draw_editor,
    rendered_lines,
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
/// - Home routes to Explorer before key events enter the nested directory.
/// - Explorer opens the Markdown file in Viewer.
/// - Test-backend rendering exposes the original document.
/// - The edit key is handled without exiting the component tree.
/// - The injected editor receives the canonical path and replaces the source.
/// - The mounted Viewer reloads and renders the edited source.
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
    let canonical_guide = fs::canonicalize(&guide).expect("the guide should canonicalize");
    let commands = Arc::new(Mutex::new(Vec::new()));
    let editor_process = EditorProcess::with_services(
        RecordingLauncher {
            commands: Arc::clone(&commands),
            outcome: TestLaunchOutcome::Success,
            replacement: Some((canonical_guide.clone(), String::from("# After edit"))),
        },
        TestEnvironment {
            visual: Some(OsString::from("configured-editor")),
            editor: None,
        },
    );
    let contexts = TestContexts::new(tree.root());
    let mut view = contexts.view_with_editor(editor_process);
    let mut terminal = Terminal::new(TestBackend::new(90, 24))?;

    draw_editor(&mut terminal, &view)?;
    let home = rendered_lines(&terminal).join("\n");
    assert!(
        home.contains("No recent Markdown files"),
        "rendered text: {home:?}"
    );

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_editor(&mut terminal, &view)?;
    assert!(rendered_lines(&terminal).join("\n").contains("> [D] docs"));
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_editor(&mut terminal, &view)?;
    assert!(rendered_lines(&terminal).join("\n").contains("guide.md"));
    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    draw_editor(&mut terminal, &view)?;
    let before_edit = rendered_lines(&terminal).join("\n");
    assert!(before_edit.contains("Before edit"));

    assert_eq!(
        view.handle_key_event(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE))?,
        KeyControl::Handled
    );
    assert_eq!(
        commands
            .lock()
            .expect("recorded commands should not be poisoned")
            .as_slice(),
        [(
            OsString::from("configured-editor"),
            vec![
                OsString::from("--"),
                canonical_guide.clone().into_os_string(),
            ],
        )]
    );
    draw_editor(&mut terminal, &view)?;
    let after_edit = rendered_lines(&terminal).join("\n");
    assert!(after_edit.contains("After edit"));

    Ok(())
}

/// Verifies external-editor failures render after the terminal session resumes.
///
/// # Example Under Test
///
/// ```text
/// missing editor
/// non-zero editor
/// e
/// ```
///
/// # Assertions
///
/// - Each injected editor failure contains its distinct diagnostic.
/// - The mounted Viewer renders the contextual failure after the edit request.
#[test]
fn workflow_renders_external_editor_failures() -> leptatui::Result<()> {
    for (label, outcome, expected) in [
        (
            "workflow-editor-missing",
            TestLaunchOutcome::NotFound,
            "failed to launch editor",
        ),
        (
            "workflow-editor-non-zero",
            TestLaunchOutcome::NonZero,
            "exited with a non-zero status",
        ),
    ] {
        let tree = TestTree::new(label);
        let guide = tree.root().join("guide.md");
        fs::write(&guide, "# Guide").expect("the Markdown file should be created");
        let canonical = fs::canonicalize(&guide).expect("the guide should canonicalize");
        let editor_process = EditorProcess::with_services(
            RecordingLauncher {
                commands: Arc::new(Mutex::new(Vec::new())),
                outcome,
                replacement: None,
            },
            TestEnvironment {
                visual: Some(OsString::from("configured-editor")),
                editor: None,
            },
        );
        let contexts = TestContexts::new(tree.root());
        let mut view = contexts.view_at_with_editor(viewer_location(&canonical), editor_process);
        let mut terminal = Terminal::new(TestBackend::new(100, 18))?;
        draw_editor(&mut terminal, &view)?;
        assert_eq!(
            view.handle_key_event(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE))?,
            KeyControl::Handled
        );
        draw_editor(&mut terminal, &view)?;
        let rendered = rendered_lines(&terminal).join("\n");
        assert!(rendered.contains(expected));
    }

    Ok(())
}
