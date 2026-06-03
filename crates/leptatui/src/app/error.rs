//! Runtime error and result types.
//!
//! This module centralizes Leptatui app-loop errors from terminal I/O and
//! asynchronous event polling tasks.

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
