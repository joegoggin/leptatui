//! Terminal session setup and cleanup.
//!
//! This module enters raw mode and the alternate screen for app execution, then
//! restores terminal state through explicit cleanup and drop guards.

use std::io::{Stdout, stdout};

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use super::Result;

/// Default Crossterm-backed terminal used by the app runner.
pub(super) type DefaultTerminal = Terminal<CrosstermBackend<Stdout>>;

/// Entered terminal state paired with cleanup flags.
pub(super) struct TerminalSession {
    /// Ratatui terminal used for all draw calls.
    pub(super) terminal: DefaultTerminal,
    /// Cleanup guard tracking terminal modes that need restoration.
    cleanup: TerminalCleanup,
}

impl TerminalSession {
    /// Enters raw mode and the alternate screen.
    ///
    /// # Returns
    ///
    /// A [`TerminalSession`] ready for rendering.
    ///
    /// # Errors
    ///
    /// Returns [`crate::app::Error::Io`] if raw mode, alternate screen entry,
    /// or terminal construction fails.
    pub(super) fn enter() -> Result<Self> {
        let mut cleanup = TerminalCleanup::default();

        enable_raw_mode()?;
        cleanup.raw_mode = true;

        if let Err(error) = execute!(stdout(), EnterAlternateScreen) {
            let _ = cleanup.restore();
            return Err(error.into());
        }
        cleanup.alternate_screen = true;

        match Terminal::new(CrosstermBackend::new(stdout())) {
            Ok(terminal) => Ok(Self { terminal, cleanup }),
            Err(error) => {
                let _ = cleanup.restore();
                Err(error.into())
            }
        }
    }

    /// Restores terminal modes for this session.
    ///
    /// # Returns
    ///
    /// An empty [`std::io::Result`] on successful cleanup.
    ///
    /// # Errors
    ///
    /// Returns [`std::io::Error`] if raw mode or alternate screen cleanup
    /// fails.
    pub(super) fn restore(&mut self) -> std::io::Result<()> {
        self.cleanup.restore()
    }
}

impl Drop for TerminalSession {
    /// Restores terminal modes when the session is dropped.
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

/// Tracks which terminal modes have been entered and still need cleanup.
#[derive(Default)]
struct TerminalCleanup {
    /// Whether raw mode is currently enabled.
    raw_mode: bool,
    /// Whether the alternate screen is currently active.
    alternate_screen: bool,
}

impl Drop for TerminalCleanup {
    /// Retries terminal cleanup when explicit restoration leaves active modes.
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

impl TerminalCleanup {
    /// Restores all active terminal modes, returning the first cleanup error.
    ///
    /// # Returns
    ///
    /// An empty [`std::io::Result`] when every active mode is restored.
    ///
    /// # Errors
    ///
    /// Returns [`std::io::Error`] if disabling raw mode or leaving the
    /// alternate screen fails.
    fn restore(&mut self) -> std::io::Result<()> {
        let mut first_error = None;

        if self.raw_mode {
            match disable_raw_mode() {
                Ok(()) => self.raw_mode = false,
                Err(error) => {
                    first_error.get_or_insert(error);
                }
            }
        }

        if self.alternate_screen {
            match execute!(stdout(), LeaveAlternateScreen) {
                Ok(()) => self.alternate_screen = false,
                Err(error) => {
                    first_error.get_or_insert(error);
                }
            }
        }

        if let Some(error) = first_error {
            Err(error)
        } else {
            Ok(())
        }
    }
}
