//! Command-line behavior tests.

use std::{env, ffi::OsString, path::PathBuf};

use clap::Parser;

use crate::{cli::Cli, initialize};

use super::support::TestTree;

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

/// Verifies startup wraps invalid command-line arguments with `anyhow` context.
///
/// # Example Under Test
///
/// ```text
/// markdown-editor docs notes
/// ```
///
/// # Assertions
///
/// - Initialization fails before workspace setup.
/// - The outer diagnostic identifies command-line parsing.
/// - The source chain retains Clap's unexpected-argument diagnostic.
#[test]
fn initialization_contextualizes_invalid_arguments() {
    let error = initialize(["markdown-editor", "docs", "notes"])
        .expect_err("initialization should reject additional roots");

    assert_eq!(error.to_string(), "failed to parse command-line arguments");
    assert!(
        error
            .chain()
            .skip(1)
            .any(|source| source.to_string().contains("unexpected argument"))
    );
}

/// Verifies startup wraps workspace validation failures with path context.
///
/// # Example Under Test
///
/// ```text
/// markdown-editor /temporary/workspace/missing
/// ```
///
/// # Assertions
///
/// - Initialization fails for the missing browsing root.
/// - The outer diagnostic includes the requested path.
/// - The source chain retains the filesystem resolution diagnostic.
#[test]
fn initialization_contextualizes_workspace_failures() {
    let tree = TestTree::new("initialization-context");
    let missing = tree.root().join("missing");
    let error = initialize([
        OsString::from("markdown-editor"),
        missing.clone().into_os_string(),
    ])
    .expect_err("initialization should reject a missing root");

    let diagnostic = error.to_string();
    assert!(diagnostic.contains("failed to initialize workspace"));
    assert!(diagnostic.contains(&missing.display().to_string()));
    assert!(error.chain().skip(1).any(|source| {
        source
            .to_string()
            .contains("failed to resolve browsing root")
    }));
}
