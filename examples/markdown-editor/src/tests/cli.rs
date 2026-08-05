//! Command-line path and startup-route behavior tests.

use std::{env, path::PathBuf};

use clap::Parser;

use crate::{absolute_file_path, cli::Cli};

/// Verifies the CLI accepts an omitted Markdown path.
///
/// # Example Under Test
///
/// ```text
/// markdown-editor
/// ```
///
/// # Assertions
///
/// - Parsing succeeds without a positional path.
/// - No startup file path is selected.
#[test]
fn cli_defaults_to_home() {
    let cli = Cli::try_parse_from(["markdown-editor"])
        .expect("the command should accept an omitted Markdown path");

    assert_eq!(cli.file_path, None);
}

/// Verifies the CLI accepts exactly one Markdown path.
///
/// # Example Under Test
///
/// ```text
/// markdown-editor docs/guide.md
/// ```
///
/// # Assertions
///
/// - Parsing succeeds with one positional path.
/// - The supplied path is retained without filesystem validation.
#[test]
fn cli_accepts_one_file_path() {
    let cli = Cli::try_parse_from(["markdown-editor", "docs/guide.md"])
        .expect("the command should accept one Markdown path");

    assert_eq!(cli.file_path, Some(PathBuf::from("docs/guide.md")));
}

/// Verifies the CLI rejects additional positional paths.
///
/// # Example Under Test
///
/// ```text
/// markdown-editor docs/guide.md notes.md
/// ```
///
/// # Assertions
///
/// - Parsing fails when two paths are supplied.
#[test]
fn cli_rejects_additional_paths() {
    let result = Cli::try_parse_from(["markdown-editor", "docs/guide.md", "notes.md"]);

    assert!(result.is_err());
}

/// Verifies startup makes relative paths absolute without requiring a file.
///
/// # Assertions
///
/// - Resolution anchors the path at the process current directory.
/// - Lexical parent components are removed.
/// - The missing target does not cause an error.
#[test]
fn startup_normalizes_relative_file_paths_without_validation() {
    let resolved = absolute_file_path(PathBuf::from("missing/../draft.md").as_path())
        .expect("lexical path resolution should not access the target");

    assert_eq!(
        resolved,
        env::current_dir()
            .expect("the current directory should resolve")
            .join("draft.md")
    );
}
