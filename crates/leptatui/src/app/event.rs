//! Async terminal event polling.
//!
//! This module bridges Crossterm's blocking event API into async app loops by
//! running terminal polling on a blocking task.

use std::time::Duration;

use crossterm::event::{self, Event};

use super::{Error, Result};

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
pub(super) async fn next_event(timeout: Duration) -> Result<Option<Event>> {
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
