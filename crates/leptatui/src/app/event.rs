//! Async terminal event polling.
//!
//! This module bridges Crossterm's blocking event API into async app loops by
//! running terminal polling on a blocking task.

use std::time::Duration;

use crossterm::event::{self, Event};

use super::{Error, Result};

/// Returns queued terminal events when one arrives before the timeout.
///
/// Event polling runs on a blocking task so async runtimes do not block on
/// Crossterm input reads. After the first event arrives, any events already
/// queued are drained into the same batch.
///
/// # Arguments
///
/// * `timeout` — Maximum time to wait for terminal input.
///
/// # Returns
///
/// A [`Vec<Event>`] containing queued events, or an empty vector after timeout.
///
/// # Errors
///
/// Returns [`Error::Io`] if polling or reading input fails. Returns
/// [`Error::EventTask`] if the blocking event task fails.
pub(super) async fn next_events(timeout: Duration) -> Result<Vec<Event>> {
    tokio::task::spawn_blocking(move || -> std::io::Result<Vec<Event>> {
        let mut events = Vec::new();
        if !event::poll(timeout)? {
            return Ok(events);
        }

        events.push(event::read()?);
        while event::poll(Duration::ZERO)? {
            events.push(event::read()?);
        }

        Ok(events)
    })
    .await?
    .map_err(Error::from)
}
