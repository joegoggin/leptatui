//! Component rendering contract.
//!
//! Components render into a [`RenderCtx`], which wraps a Ratatui frame and the
//! rectangular area currently assigned to the component.

mod contract;
mod model;

pub use contract::Component;
pub use model::RenderCtx;
