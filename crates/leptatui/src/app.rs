//! Terminal app runner.
//!
//! This module owns terminal setup, event polling, root rendering, and cleanup
//! for Leptatui applications. It adapts either an [`AppRoot`] implementation or
//! a [`Component`] to a managed Ratatui/Crossterm terminal session.

use std::{
    io::{Stdout, stdout},
    time::Duration,
};

use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Frame, Terminal, backend::CrosstermBackend};

use crate::component::{Component, RenderCtx};

/// Time between event polls when no input is available.
const DEFAULT_REDRAW_INTERVAL: Duration = Duration::from_millis(16);

/// Result type returned by Leptatui runtime APIs.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors returned by Leptatui runtime APIs.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Terminal setup, rendering, input, or cleanup failed.
    #[error("terminal I/O failed")]
    Io(#[from] std::io::Error),

    /// Tokio failed to join the blocking event polling task.
    #[error("event polling task failed")]
    EventTask(#[from] tokio::task::JoinError),
}

impl From<std::convert::Infallible> for Error {
    /// Converts an impossible error into a runtime [`Error`].
    ///
    /// # Arguments
    ///
    /// * `error` — Uninhabited error value.
    ///
    /// # Returns
    ///
    /// An [`Error`] value, though this function can never be called.
    fn from(error: std::convert::Infallible) -> Self {
        match error {}
    }
}

/// Controls whether the app runner keeps looping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppControl {
    /// Continue running the app loop.
    Continue,
    /// Exit the app loop and restore the terminal.
    Exit,
}

/// Runtime adapter consumed by `App`.
pub trait AppRoot {
    /// Renders the current root state into the Ratatui frame.
    ///
    /// # Arguments
    ///
    /// * `frame` — Ratatui frame for the current draw pass.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if rendering through the terminal backend fails.
    fn render(&mut self, frame: &mut Frame<'_>) -> Result<()>;

    /// Handles a terminal event.
    ///
    /// # Arguments
    ///
    /// * `_event` — Crossterm event emitted by the terminal.
    ///
    /// # Returns
    ///
    /// An [`AppControl`] value indicating whether the app loop should continue.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if event handling performs terminal I/O that fails.
    fn handle_event(&mut self, _event: Event) -> Result<AppControl> {
        Ok(AppControl::Continue)
    }
}

impl<T> AppRoot for T
where
    T: Component,
{
    /// Renders a [`Component`] through a full-frame [`RenderCtx`].
    ///
    /// # Arguments
    ///
    /// * `frame` — Ratatui frame for the current draw pass.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on success.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if component rendering performs terminal I/O that fails.
    fn render(&mut self, frame: &mut Frame<'_>) -> Result<()> {
        let mut ctx = RenderCtx::new(frame);
        Component::render(self, &mut ctx)
    }

    /// Handles a terminal event by delegating to the wrapped [`Component`].
    ///
    /// # Arguments
    ///
    /// * `event` — Crossterm event emitted by the terminal.
    ///
    /// # Returns
    ///
    /// An [`AppControl`] value indicating whether the app loop should continue.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if component event handling performs terminal I/O
    /// that fails.
    fn handle_event(&mut self, event: Event) -> Result<AppControl> {
        Component::handle_event(self, event)
    }
}

/// Runs a root component in a managed terminal session.
#[derive(Debug)]
pub struct App<R> {
    /// Root component or runtime adapter rendered by the app loop.
    root: R,
    /// Polling timeout that also controls idle redraw cadence.
    redraw_interval: Duration,
}

impl<R> App<R> {
    /// Creates an app runner for a root component.
    ///
    /// # Arguments
    ///
    /// * `root` — Root component or [`AppRoot`] adapter to render.
    ///
    /// # Returns
    ///
    /// An [`App`] configured with the default redraw interval.
    pub fn new(root: R) -> Self {
        Self {
            root,
            redraw_interval: DEFAULT_REDRAW_INTERVAL,
        }
    }

    /// Overrides the polling timeout that drives periodic redraws.
    ///
    /// # Arguments
    ///
    /// * `redraw_interval` — Non-zero event polling timeout and idle redraw
    ///   cadence.
    ///
    /// # Returns
    ///
    /// An [`App`] configured with the provided redraw interval.
    ///
    /// # Panics
    ///
    /// Panics if `redraw_interval` is zero.
    pub fn with_redraw_interval(mut self, redraw_interval: Duration) -> Self {
        assert!(
            !redraw_interval.is_zero(),
            "redraw interval must be greater than zero"
        );
        self.redraw_interval = redraw_interval;
        self
    }
}

impl<R> App<R>
where
    R: AppRoot,
{
    /// Runs the app until the root requests exit or a runtime error occurs.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] on successful exit and terminal cleanup.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if terminal setup, rendering, input, or cleanup
    /// fails. Returns [`Error::EventTask`] if the blocking event task fails.
    pub async fn run(mut self) -> Result<()> {
        let mut session = TerminalSession::enter()?;

        let loop_result = self.run_loop(&mut session).await;
        let restore_result = session.restore();

        loop_result?;
        restore_result?;

        Ok(())
    }

    /// Runs the draw and event polling loop for an entered terminal session.
    ///
    /// # Arguments
    ///
    /// * `session` — Active terminal session to draw into and eventually leave.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] when the root requests exit.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if drawing, event polling, or event reading fails.
    /// Returns [`Error::EventTask`] if the blocking event task fails.
    async fn run_loop(&mut self, session: &mut TerminalSession) -> Result<()> {
        loop {
            draw_root(&mut self.root, &mut session.terminal)?;

            if let Some(event) = next_event(self.redraw_interval).await?
                && self.root.handle_event(event)? == AppControl::Exit
            {
                break;
            }
        }

        Ok(())
    }
}

/// Draws a root application into the terminal.
///
/// # Arguments
///
/// * `root` — Root application state to render.
/// * `terminal` — Ratatui terminal backend receiving the draw call.
///
/// # Returns
///
/// An empty [`Result`] on success.
///
/// # Errors
///
/// Returns [`Error::Io`] if the terminal draw call fails or root rendering
/// fails through terminal I/O.
fn draw_root<R>(root: &mut R, terminal: &mut DefaultTerminal) -> Result<()>
where
    R: AppRoot,
{
    let mut render_result: Result<()> = Ok(());

    terminal.draw(|frame| {
        render_result = root.render(frame);
    })?;

    render_result
}

/// Returns the next terminal event if one arrives before the timeout.
///
/// Event polling runs on a blocking task so async runtimes do not block on
/// Crossterm input reads.
///
/// # Arguments
///
/// * `timeout` — Maximum time to wait for terminal input.
///
/// # Returns
///
/// An [`Option<Event>`] containing the next event when input is ready.
///
/// # Errors
///
/// Returns [`Error::Io`] if polling or reading input fails. Returns
/// [`Error::EventTask`] if the blocking event task fails.
async fn next_event(timeout: Duration) -> Result<Option<Event>> {
    tokio::task::spawn_blocking(move || {
        if event::poll(timeout)? {
            event::read().map(Some)
        } else {
            Ok(None)
        }
    })
    .await?
    .map_err(Error::from)
}

/// Default Crossterm-backed terminal used by the app runner.
type DefaultTerminal = Terminal<CrosstermBackend<Stdout>>;

/// Entered terminal state paired with cleanup flags.
struct TerminalSession {
    /// Ratatui terminal used for all draw calls.
    terminal: DefaultTerminal,
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
    /// Returns [`Error::Io`] if raw mode, alternate screen entry, or terminal
    /// construction fails.
    fn enter() -> Result<Self> {
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
    /// Returns [`std::io::Error`] if raw mode or alternate screen cleanup fails.
    fn restore(&mut self) -> std::io::Result<()> {
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
            if let Err(error) = disable_raw_mode() {
                first_error.get_or_insert(error);
            }
            self.raw_mode = false;
        }

        if self.alternate_screen {
            if let Err(error) = execute!(stdout(), LeaveAlternateScreen) {
                first_error.get_or_insert(error);
            }
            self.alternate_screen = false;
        }

        if let Some(error) = first_error {
            Err(error)
        } else {
            Ok(())
        }
    }
}
