//! Behavior tests for the Markdown editor.
//!
//! Coverage is grouped by application boundary and includes an end-to-end
//! workflow that exercises explorer, preview, editor, and rendering behavior.
//!
//! # Modules
//!
//! - [`app`] — Application routing and responsive rendering.
//! - [`cli`] — Command-line parsing and root selection.
//! - [`editor_process`] — Editor configuration and process construction.
//! - [`filesystem`] — Anchored discovery and filesystem failure behavior.
//! - [`pages`] — Page-specific keyboard handling and rendering.
//! - [`recent_files`] — Persistent MRU ordering and recoverable storage failures.
//! - [`support`] — Shared fixtures, mocks, and rendering helpers.
//! - [`workflow`] — End-to-end non-interactive application coverage.

mod app;
mod cli;
mod editor_process;
mod filesystem;
mod pages;
mod recent_files;
mod support;
mod workflow;
