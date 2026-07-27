//! Behavior tests for the Markdown editor.
//!
//! Coverage is grouped by application boundary and includes an end-to-end
//! workflow that exercises explorer, preview, editor, and rendering behavior.
//!
//! # Modules
//!
//! - [`cli`] — Command-line parsing and root selection.
//! - [`controller`] — Explorer, preview, and edit state transitions.
//! - [`editor_process`] — Editor configuration and process construction.
//! - [`filesystem`] — Anchored discovery and filesystem failure behavior.
//! - [`recent_files`] — Persistent MRU ordering and recoverable storage failures.
//! - [`support`] — Shared fixtures, mocks, and rendering helpers.
//! - [`ui`] — Keyboard handling and responsive rendering.
//! - [`workflow`] — End-to-end non-interactive application coverage.

mod cli;
mod controller;
mod editor_process;
mod filesystem;
mod recent_files;
mod support;
mod ui;
mod workflow;
