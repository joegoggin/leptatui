//! Terminal session setup and cleanup.
//!
//! This module enters raw mode and the alternate screen for app execution,
//! restores terminal state through explicit cleanup and drop guards, and wraps
//! the process panic hook so diagnostics appear after terminal restoration.

use std::{
    io::{Stdout, stdout},
    panic::PanicHookInfo,
    sync::{Arc, Mutex, MutexGuard, OnceLock},
};

use crossterm::{
    cursor::SetCursorStyle,
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::terminal_image::TerminalImageSupport;

use super::Result;

/// Default Crossterm-backed terminal used by the app runner.
pub(super) type DefaultTerminal = Terminal<CrosstermBackend<Stdout>>;

/// Entered terminal state paired with cleanup flags.
pub(super) struct TerminalSession {
    /// Ratatui terminal used for all draw calls.
    pub(super) terminal: DefaultTerminal,
    /// Terminal image support detected for this session.
    pub(super) terminal_images: TerminalImageSupport,
    /// Cleanup guard tracking terminal modes that need restoration.
    cleanup: TerminalCleanup,
}

/// Shared callback type used to preserve the process panic hook.
type PanicHook = dyn for<'a> Fn(&PanicHookInfo<'a>) + Send + Sync + 'static;

/// Restores the previous process panic hook after a managed app finishes.
pub(super) struct PanicHookGuard {
    /// Previous hook invoked after terminal restoration.
    previous: Arc<PanicHook>,
    /// Process-wide lock preventing overlapping scoped hook replacement.
    _lock: MutexGuard<'static, ()>,
}

impl TerminalSession {
    /// Enters raw mode, the alternate screen, and mouse capture.
    ///
    /// # Returns
    ///
    /// A [`TerminalSession`] ready for rendering.
    ///
    /// # Errors
    ///
    /// Returns [`crate::app::Error::Io`] if raw mode, alternate screen entry,
    /// mouse capture, or terminal construction fails.
    pub(super) fn enter() -> Result<Self> {
        let cleanup = TerminalCleanup::default();

        enable_raw_mode()?;
        cleanup.update(|state| state.raw_mode = true);

        if let Err(error) = execute!(stdout(), EnterAlternateScreen) {
            let _ = cleanup.restore();
            return Err(error.into());
        }
        cleanup.update(|state| {
            state.alternate_screen = true;
            state.cursor_style = true;
        });

        if let Err(error) = execute!(stdout(), EnableMouseCapture) {
            let _ = cleanup.restore();
            return Err(error.into());
        }
        cleanup.update(|state| state.mouse_capture = true);

        let terminal_images = TerminalImageSupport::query_stdio();

        match Terminal::new(CrosstermBackend::new(stdout())) {
            Ok(terminal) => Ok(Self {
                terminal,
                terminal_images,
                cleanup,
            }),
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
    /// Returns [`std::io::Error`] if cursor restoration, mouse capture, raw
    /// mode, or alternate screen cleanup fails.
    pub(super) fn restore(&mut self) -> std::io::Result<()> {
        self.cleanup.restore()
    }

    /// Installs a panic hook that restores this terminal before diagnostics.
    ///
    /// # Returns
    ///
    /// A [`PanicHookGuard`] that restores the previous hook when dropped.
    pub(super) fn install_panic_hook(&self) -> PanicHookGuard {
        PanicHookGuard::install(self.cleanup.clone())
    }
}

impl Drop for TerminalSession {
    /// Restores terminal modes when the session is dropped.
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

/// Shared terminal modes that may require cleanup.
#[derive(Default)]
struct TerminalCleanupState {
    /// Whether raw mode is currently enabled.
    raw_mode: bool,
    /// Whether the alternate screen is currently active.
    alternate_screen: bool,
    /// Whether the cursor style should be restored to the user's default.
    cursor_style: bool,
    /// Whether terminal mouse capture is currently active.
    mouse_capture: bool,
    /// Test-only callback observing cleanup before panic diagnostics.
    #[cfg(test)]
    restore_observer: Option<Arc<dyn Fn() + Send + Sync>>,
}

/// Tracks which terminal modes have been entered and still need cleanup.
#[derive(Clone, Default)]
struct TerminalCleanup {
    /// Shared state available to both normal cleanup and the panic hook.
    state: Arc<Mutex<TerminalCleanupState>>,
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
    /// Returns [`std::io::Error`] if cursor restoration, mouse capture, raw
    /// mode, or alternate screen cleanup fails.
    fn restore(&self) -> std::io::Result<()> {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        let mut first_error = None;

        #[cfg(test)]
        if let Some(observer) = &state.restore_observer {
            observer();
        }

        if state.cursor_style {
            match execute!(stdout(), SetCursorStyle::DefaultUserShape) {
                Ok(()) => state.cursor_style = false,
                Err(error) => {
                    first_error.get_or_insert(error);
                }
            }
        }

        if state.mouse_capture {
            match execute!(stdout(), DisableMouseCapture) {
                Ok(()) => state.mouse_capture = false,
                Err(error) => {
                    first_error.get_or_insert(error);
                }
            }
        }

        if state.raw_mode {
            match disable_raw_mode() {
                Ok(()) => state.raw_mode = false,
                Err(error) => {
                    first_error.get_or_insert(error);
                }
            }
        }

        if state.alternate_screen {
            match execute!(stdout(), LeaveAlternateScreen) {
                Ok(()) => state.alternate_screen = false,
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

    /// Mutates the shared cleanup flags after a terminal transition.
    ///
    /// # Arguments
    ///
    /// * `update` — Callback applying one or more state changes.
    fn update(&self, update: impl FnOnce(&mut TerminalCleanupState)) {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        update(&mut state);
    }
}

impl PanicHookGuard {
    /// Installs a terminal-restoring wrapper around the current panic hook.
    ///
    /// # Arguments
    ///
    /// * `cleanup` — Shared cleanup state for the active terminal session.
    ///
    /// # Returns
    ///
    /// A [`PanicHookGuard`] retaining the previous hook and replacement lock.
    fn install(cleanup: TerminalCleanup) -> Self {
        static PANIC_HOOK_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

        let lock = PANIC_HOOK_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let previous: Arc<PanicHook> = Arc::from(std::panic::take_hook());
        let chained = Arc::clone(&previous);
        std::panic::set_hook(Box::new(move |panic| {
            let _ = cleanup.restore();
            chained(panic);
        }));

        Self {
            previous,
            _lock: lock,
        }
    }
}

impl Drop for PanicHookGuard {
    /// Restores the hook that was active before the managed app started.
    fn drop(&mut self) {
        let previous = Arc::clone(&self.previous);
        std::panic::set_hook(Box::new(move |panic| previous(panic)));
    }
}

#[cfg(test)]
/// Unit tests for panic-time terminal restoration.
mod tests {
    use std::sync::Mutex;

    use super::*;

    /// Verifies panic cleanup runs before the previously installed hook.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// managed panic hook -> terminal cleanup -> previous panic hook
    /// ```
    ///
    /// # Assertions
    ///
    /// - The synthetic panic is caught after invoking the managed hook.
    /// - Terminal cleanup is observed before the previous hook.
    #[test]
    fn panic_hook_restores_terminal_before_diagnostics() {
        static TEST_HOOK_LOCK: Mutex<()> = Mutex::new(());

        let _test_lock = TEST_HOOK_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let original = std::panic::take_hook();
        let order = Arc::new(Mutex::new(Vec::new()));
        let previous_order = Arc::clone(&order);
        std::panic::set_hook(Box::new(move |_| {
            previous_order
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .push("previous");
        }));

        let cleanup = TerminalCleanup::default();
        let cleanup_order = Arc::clone(&order);
        cleanup.update(|state| {
            state.restore_observer = Some(Arc::new(move || {
                cleanup_order
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .push("restore");
            }));
        });

        let guard = PanicHookGuard::install(cleanup);
        let panic = std::panic::catch_unwind(|| panic!("synthetic panic"));
        drop(guard);
        let _ = std::panic::take_hook();
        std::panic::set_hook(original);

        assert!(panic.is_err());
        assert!(
            order
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .windows(2)
                .any(|events| events == ["restore", "previous"]),
        );
    }
}
