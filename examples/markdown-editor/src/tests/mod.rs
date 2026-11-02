//! Behavior tests for the Markdown editor.
//!
//! Coverage is grouped by application boundary and exercises routing,
//! persistence, keyboard handling, and representative rendering behavior.
//!
//! # Modules
//!
//! - [`app`] — Application routing and responsive rendering.
//! - [`cli`] — Command-line parsing and startup-path normalization.
//! - [`pages`] — Page-specific keyboard handling and rendering.
//! - [`recent_files`] — Persistent MRU ordering and propagated storage failures.
//! - [`support`] — Shared fixtures, mocks, and rendering helpers.

mod app;
mod cli;
mod pages;
mod recent_files;
mod support;
