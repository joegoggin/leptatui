//! Shared contracts and infrastructure for terminal views.
//!
//! # Modules
//!
//! - [`any_view`] — Type-erased view storage and forwarding.
//! - [`capabilities`] — Optional view capability traits and implementations.
//! - [`contract`] — Core object-safe view protocol.
//! - [`conversion`] — Single-view and collection conversion traits.
//! - [`events`] — Default key handling and focus movement.
//! - [`metadata`] — Selector, focus, and scrolling metadata.
//! - [`render`] — Shared style, height, and focus geometry helpers.

pub(crate) mod any_view;
pub(crate) mod capabilities;
pub(crate) mod contract;
pub(crate) mod conversion;
pub(crate) mod events;
pub(crate) mod measurement;
pub(crate) mod metadata;
pub(crate) mod render;
