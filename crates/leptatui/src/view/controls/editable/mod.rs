//! Shared editing behavior and concrete editable controls.
//!
//! # Modules
//!
//! - [`input`] — Single-line controlled input view.
//! - [`insert`] — Insert-mode editing and pending-key handling.
//! - [`model`] — Shared editable view state and callbacks.
//! - [`movement`] — UTF-8-safe cursor and text movement helpers.
//! - [`normal`] — Normal-mode command dispatch.
//! - [`render`] — Editable rendering, scrolling, and geometry.
//! - [`state`] — Vim mode, history, cursor, and selection state.
//! - [`text_area`] — Multiline controlled text-area view.
//! - [`visual`] — Visual-mode selection and editing behavior.

pub(crate) mod input;
pub(crate) mod insert;
pub(crate) mod model;
pub(crate) mod movement;
pub(crate) mod normal;
pub(crate) mod render;
pub(crate) mod state;
pub(crate) mod text_area;
pub(crate) mod visual;
