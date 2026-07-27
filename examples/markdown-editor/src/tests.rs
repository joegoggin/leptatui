//! Startup tests for the Markdown editor.

use std::{
    env, fs,
    path::PathBuf,
    process,
    time::{SystemTime, UNIX_EPOCH},
};

use clap::Parser;

use crate::{
    cli::Cli, controller::Controller, editor_process::EditorProcess, filesystem::FileSystem,
};

/// Returns a process-local temporary path for one filesystem test.
///
/// # Arguments
///
/// * `label` — Readable scenario name included in the temporary path.
///
/// # Returns
///
/// A [`PathBuf`] beneath the operating system temporary directory.
fn temporary_path(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after the Unix epoch")
        .as_nanos();

    env::temp_dir().join(format!(
        "leptatui-markdown-editor-{label}-{}-{nonce}",
        process::id()
    ))
}

/// Verifies the CLI accepts no browsing root and uses the current directory.
///
/// # Example Under Test
///
/// ```text
/// markdown-editor
/// ```
///
/// # Assertions
///
/// - Parsing succeeds without a positional root.
/// - Root resolution returns the process current directory.
#[test]
fn cli_defaults_to_current_directory() {
    let cli = Cli::try_parse_from(["markdown-editor"])
        .expect("the command should accept an omitted browsing root");

    assert_eq!(
        cli.requested_root()
            .expect("the current directory should be readable"),
        env::current_dir().expect("the current directory should be readable")
    );
}

/// Verifies the CLI accepts exactly one explicit browsing root.
///
/// # Example Under Test
///
/// ```text
/// markdown-editor docs
/// ```
///
/// # Assertions
///
/// - Parsing succeeds with one positional root.
/// - The resolved root equals `docs`.
#[test]
fn cli_accepts_one_explicit_root() {
    let cli = Cli::try_parse_from(["markdown-editor", "docs"])
        .expect("the command should accept one browsing root");

    assert_eq!(
        cli.requested_root()
            .expect("an explicit root should not query the current directory"),
        PathBuf::from("docs")
    );
}

/// Verifies the CLI rejects additional positional roots.
///
/// # Example Under Test
///
/// ```text
/// markdown-editor docs notes
/// ```
///
/// # Assertions
///
/// - Parsing fails when two roots are supplied.
#[test]
fn cli_rejects_additional_roots() {
    let result = Cli::try_parse_from(["markdown-editor", "docs", "notes"]);

    assert!(result.is_err());
}

/// Verifies filesystem validation returns a canonical directory workspace.
///
/// # Example Under Test
///
/// ```text
/// <temporary-directory>/workspace
/// ```
///
/// # Assertions
///
/// - The temporary directory is created successfully.
/// - Root validation succeeds.
/// - The workspace root equals the canonical temporary-directory path.
#[test]
fn filesystem_canonicalizes_valid_directory() {
    let root = temporary_path("valid-root");
    fs::create_dir(&root).expect("the temporary directory should be created");
    let expected = fs::canonicalize(&root).expect("the temporary directory should canonicalize");

    let workspace = FileSystem::new()
        .validate_root(&root)
        .expect("a directory should be a valid browsing root");

    assert_eq!(workspace.root(), expected);
    fs::remove_dir(&root).expect("the temporary directory should be removed");
}

/// Verifies filesystem validation rejects regular files as browsing roots.
///
/// # Example Under Test
///
/// ```text
/// <temporary-directory>/not-a-directory.md
/// ```
///
/// # Assertions
///
/// - The temporary file is created successfully.
/// - Validation returns `InvalidInput`.
/// - The diagnostic identifies that the root is not a directory.
#[test]
fn filesystem_rejects_regular_file() {
    let root = temporary_path("regular-file.md");
    fs::write(&root, "# Not a directory").expect("the temporary file should be created");

    let error = FileSystem::new()
        .validate_root(&root)
        .expect_err("a regular file should not be a valid browsing root");

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(error.to_string().contains("not a directory"));
    fs::remove_file(&root).expect("the temporary file should be removed");
}

/// Verifies controller initialization rejects a missing root before UI startup.
///
/// # Example Under Test
///
/// ```text
/// markdown-editor <missing-temporary-path>
/// ```
///
/// # Assertions
///
/// - Controller initialization fails with `NotFound`.
/// - The diagnostic contains the missing path.
///
/// # Why
///
/// The binary must complete controller initialization before constructing the
/// Leptatui app, preventing invalid roots from entering managed terminal mode.
#[test]
fn controller_rejects_missing_root_before_ui_startup() {
    let root = temporary_path("missing-root");

    let error = Controller::initialize(&root, FileSystem::new(), EditorProcess::new())
        .expect_err("a missing root should fail controller initialization");

    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    assert!(error.to_string().contains(&root.display().to_string()));
}
