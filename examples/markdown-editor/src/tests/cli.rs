//! Command-line behavior tests.

use std::{env, path::PathBuf};

use clap::Parser;

use crate::core::Cli;

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
