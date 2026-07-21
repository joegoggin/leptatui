//! Integration tests for generated components.
//!
//! # Modules
//!
//! - [`support`] — Shared event and rendering helpers.
//! - [`suite`] — Component macro fixtures and behavioral tests.

mod support;

#[path = "component_macro/mod.rs"]
mod suite;
