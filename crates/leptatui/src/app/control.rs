//! App-loop control flow values.
//!
//! This module defines the signal returned by event handlers to continue or
//! exit the managed terminal loop.

/// Controls whether the app runner keeps looping.
///
/// Control callbacks may return `()` instead, which converts to
/// [`AppControl::Continue`]. Return [`AppControl::Exit`] to quit explicitly.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppControl {
    /// Continue running the app loop.
    Continue,
    /// Exit the app loop and restore the terminal.
    Exit,
}

impl From<()> for AppControl {
    /// Converts a callback's unit return into continued execution.
    ///
    /// # Arguments
    ///
    /// * `_value` — Unit returned by a callback.
    ///
    /// # Returns
    ///
    /// An [`AppControl::Continue`] value.
    fn from(_value: ()) -> Self {
        Self::Continue
    }
}
