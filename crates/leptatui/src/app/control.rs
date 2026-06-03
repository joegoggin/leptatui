//! App-loop control flow values.
//!
//! This module defines the signal returned by event handlers to continue or
//! exit the managed terminal loop.

/// Controls whether the app runner keeps looping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppControl {
    /// Continue running the app loop.
    Continue,
    /// Exit the app loop and restore the terminal.
    Exit,
}
