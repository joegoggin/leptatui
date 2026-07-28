//! Runtime error and result types.
//!
//! This module centralizes Leptatui app-loop errors from terminal I/O,
//! asynchronous event polling tasks, link activation, and component-owned
//! application initialization.

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

    /// The operating system could not open an activated link target.
    #[error("failed to open link target `{target}`: {source}")]
    LinkOpen {
        /// Target that could not be opened.
        target: String,
        /// Underlying filesystem or process-launch failure.
        #[source]
        source: std::io::Error,
    },

    /// A component-owned application failure returned after the app exits.
    #[error("application failed")]
    Application(#[source] Box<dyn std::error::Error + Send + Sync>),
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
