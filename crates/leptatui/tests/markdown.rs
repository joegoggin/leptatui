//! Integration tests for Markdown conversion and rendering.
//!
//! # Modules
//!
//! - [`support`] — Shared terminal rendering helpers.
//! - [`suite`] — Markdown parsing and rendering fixtures.

mod support;

#[path = "markdown/mod.rs"]
mod suite;
