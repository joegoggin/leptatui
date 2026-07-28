//! Editor command resolution and process-boundary tests.

use std::{
    ffi::OsString,
    fs, io,
    sync::{Arc, Mutex},
};

use crate::services::EditorProcess;

use super::support::{RecordingLauncher, TestEnvironment, TestLaunchOutcome, TestTree};

/// Verifies `VISUAL` takes precedence and quoted arguments are parsed safely.
///
/// # Example Under Test
///
/// ```text
/// VISUAL="custom-editor --wait --option 'two words'"
/// EDITOR=ignored-editor
/// ```
///
/// # Assertions
///
/// - The injected launcher reports a successful edit.
/// - The program comes from `VISUAL`, not `EDITOR`.
/// - Quoted words remain one argument without invoking a shell.
/// - One `--` separator precedes the absolute Markdown path.
#[test]
fn editor_process_prefers_visual_and_parses_arguments() {
    let tree = TestTree::new("editor-command");
    let markdown = tree.root().join("-guide with spaces.md");
    fs::write(&markdown, "# Guide").expect("the Markdown file should be created");
    let absolute_markdown =
        fs::canonicalize(&markdown).expect("the Markdown file should canonicalize");
    let commands = Arc::new(Mutex::new(Vec::new()));
    let process = EditorProcess::with_services(
        RecordingLauncher {
            commands: Arc::clone(&commands),
            outcome: TestLaunchOutcome::Success,
            replacement: None,
        },
        TestEnvironment {
            visual: Some(OsString::from("custom-editor --wait --option 'two words'")),
            editor: Some(OsString::from("ignored-editor")),
        },
    );

    process
        .edit(&absolute_markdown)
        .expect("the injected editor launch should succeed");

    assert_eq!(
        commands
            .lock()
            .expect("recorded commands should not be poisoned")
            .as_slice(),
        [(
            OsString::from("custom-editor"),
            vec![
                OsString::from("--wait"),
                OsString::from("--option"),
                OsString::from("two words"),
                OsString::from("--"),
                absolute_markdown.into_os_string()
            ]
        )]
    );
}

/// Verifies `EDITOR` and `vi` provide deterministic fallback commands.
///
/// # Example Under Test
///
/// ```text
/// VISUAL="   " EDITOR="configured-editor --wait --"
/// VISUAL unset, EDITOR unset
/// ```
///
/// # Assertions
///
/// - A whitespace-only `VISUAL` value is skipped in favor of `EDITOR`.
/// - An existing trailing `--` separator is not duplicated.
/// - Unset editor variables fall back to `vi`.
/// - Both commands receive the same canonical absolute path.
#[test]
fn editor_process_uses_editor_then_vi_fallback() {
    let tree = TestTree::new("editor-fallbacks");
    let markdown = tree.root().join("guide.md");
    fs::write(&markdown, "# Guide").expect("the Markdown file should be created");
    let absolute_markdown =
        fs::canonicalize(&markdown).expect("the Markdown file should canonicalize");
    let commands = Arc::new(Mutex::new(Vec::new()));
    let launcher = RecordingLauncher {
        commands: Arc::clone(&commands),
        outcome: TestLaunchOutcome::Success,
        replacement: None,
    };
    let configured = EditorProcess::with_services(
        launcher.clone(),
        TestEnvironment {
            visual: Some(OsString::from("   ")),
            editor: Some(OsString::from("configured-editor --wait --")),
        },
    );
    let fallback = EditorProcess::with_services(launcher, TestEnvironment::default());

    configured
        .edit(&absolute_markdown)
        .expect("the configured editor should succeed");
    fallback
        .edit(&absolute_markdown)
        .expect("the fallback editor should succeed");

    assert_eq!(
        commands
            .lock()
            .expect("recorded commands should not be poisoned")
            .as_slice(),
        [
            (
                OsString::from("configured-editor"),
                vec![
                    OsString::from("--wait"),
                    OsString::from("--"),
                    absolute_markdown.clone().into_os_string()
                ]
            ),
            (
                OsString::from("vi"),
                vec![OsString::from("--"), absolute_markdown.into_os_string()]
            )
        ]
    );
}

/// Verifies malformed editor configuration fails without running a shell.
///
/// # Example Under Test
///
/// ```text
/// VISUAL="editor 'unterminated"
/// ```
///
/// # Assertions
///
/// - Editing returns an invalid-input error identifying `VISUAL`.
/// - No process command reaches the injected launcher.
#[test]
fn editor_process_rejects_malformed_configuration() {
    let tree = TestTree::new("editor-malformed");
    let markdown = tree.root().join("guide.md");
    fs::write(&markdown, "# Guide").expect("the Markdown file should be created");
    let commands = Arc::new(Mutex::new(Vec::new()));
    let process = EditorProcess::with_services(
        RecordingLauncher {
            commands: Arc::clone(&commands),
            outcome: TestLaunchOutcome::Success,
            replacement: None,
        },
        TestEnvironment {
            visual: Some(OsString::from("editor 'unterminated")),
            editor: None,
        },
    );

    let error = process
        .edit(&markdown)
        .expect_err("malformed editor configuration should fail");

    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    assert!(error.to_string().contains("VISUAL"));
    assert!(
        commands
            .lock()
            .expect("recorded commands should not be poisoned")
            .is_empty()
    );
}
