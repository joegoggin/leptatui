//! Command-line parsing for the Markdown editor.
//!
//! The command accepts one optional browsing root and resolves an omitted root
//! from the process current directory before application initialization.

use std::{env, io, path::PathBuf};

use clap::Parser;

/// Command-line arguments accepted by the Markdown editor.
#[derive(Clone, Debug, Parser)]
#[command(
    name = "markdown-editor",
    about = "Browse and edit Markdown files in a terminal workspace"
)]
pub(crate) struct Cli {
    /// Optional directory used as the browsing root.
    #[arg(value_name = "ROOT")]
    root: Option<PathBuf>,
}

impl Cli {
    /// Returns the requested root or the process current directory.
    ///
    /// # Returns
    ///
    /// A [`PathBuf`] containing the root requested by the user.
    ///
    /// # Errors
    ///
    /// Returns [`io::Error`] if the current directory cannot be read when no
    /// explicit root was supplied.
    pub(crate) fn requested_root(&self) -> io::Result<PathBuf> {
        match &self.root {
            Some(root) => Ok(root.clone()),
            None => env::current_dir(),
        }
    }
}
