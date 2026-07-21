//! Terminal image backend support.
//!
//! Protocol detection, image caching, and deterministic fallback rendering are
//! kept private behind this crate-level facade.
//!
//! # Modules
//!
//! - [`backend`] - Protocol detection and state for the active render pass.
//! - [`cache`] - Decoded image and protocol cache management.
//! - [`fallback`] - Deterministic text rendering when images are unavailable.

mod backend;
mod cache;
mod fallback;

pub(crate) use backend::TerminalImageSupport;
pub(crate) use fallback::{
    TerminalImageFallback, TerminalImageRenderOutcome, render_terminal_image_fallback,
};

#[cfg(test)]
mod tests;
