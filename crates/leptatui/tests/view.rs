//! Integration tests for the built-in view system.
//!
//! # Modules
//!
//! - [`support`] — Shared event, focus, and rendering helpers.
//! - [`suite`] — Built-in view behavior and rendering tests.

mod support;

#[path = "view/mod.rs"]
mod suite;
