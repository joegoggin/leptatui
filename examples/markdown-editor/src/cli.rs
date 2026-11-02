//! Command-line parsing for the Markdown editor.
//!
//! The command accepts one optional Markdown path to open at startup.

use std::path::PathBuf;

use clap::Parser;

/// Command-line arguments accepted by the Markdown editor.
#[derive(Clone, Debug, Parser)]
#[command(
    name = "markdown-editor",
    about = "Browse and edit Markdown files in a terminal"
)]
pub(crate) struct Cli {
    /// Optional Markdown file opened when the application starts.
    #[arg(value_name = "FILE_PATH")]
    pub(crate) file_path: Option<PathBuf>,
}
