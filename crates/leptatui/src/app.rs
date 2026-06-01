//! Terminal app runner.

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
    /// Render the current root state into the Ratatui frame.
    fn render(&mut self, frame: &mut Frame<'_>) -> Result<()>;

    /// Handle a terminal event.
    fn handle_event(&mut self, _event: Event) -> Result<AppControl> {
        Ok(AppControl::Continue)
    }
}

impl<T> AppRoot for T
where
    T: Component,
{
    fn render(&mut self, frame: &mut Frame<'_>) -> Result<()> {
        let mut ctx = RenderCtx::new(frame);
        Component::render(self, &mut ctx)
    }

    fn handle_event(&mut self, event: Event) -> Result<AppControl> {
        Component::handle_event(self, event)
    }
}

/// Runs a root component in a managed terminal session.
#[derive(Debug)]
pub struct App<R> {
    root: R,
    redraw_interval: Duration,
}

impl<R> App<R> {
    /// Create an app runner for a root component.
    pub fn new(root: R) -> Self {
        Self {
            root,
            redraw_interval: DEFAULT_REDRAW_INTERVAL,
        }
    }

    /// Override the polling timeout that drives periodic redraws.
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
    /// Run the app until the root requests exit or a runtime error occurs.
    pub async fn run(mut self) -> Result<()> {
        let mut session = TerminalSession::enter()?;

        let loop_result = self.run_loop(&mut session).await;
        let restore_result = session.restore();

        loop_result?;
        restore_result?;

        Ok(())
    }

    async fn run_loop(&mut self, session: &mut TerminalSession) -> Result<()> {
        loop {
            let mut render_result: Result<()> = Ok(());

            session.terminal.draw(|frame| {
                render_result = self.root.render(frame);
            })?;
            render_result?;

            if let Some(event) = next_event(self.redraw_interval).await? {
                if self.root.handle_event(event)? == AppControl::Exit {
                    break;
                }
            }
        }

        Ok(())
    }
}

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

type DefaultTerminal = Terminal<CrosstermBackend<Stdout>>;

struct TerminalSession {
    terminal: DefaultTerminal,
    cleanup: TerminalCleanup,
}

impl TerminalSession {
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

    fn restore(&mut self) -> std::io::Result<()> {
        self.cleanup.restore()
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

#[derive(Default)]
struct TerminalCleanup {
    raw_mode: bool,
    alternate_screen: bool,
}

impl TerminalCleanup {
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
