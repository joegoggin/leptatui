//! Component rendering contract.
//!
//! Components render into a [`RenderCtx`], which wraps a Ratatui frame and the
//! rectangular area currently assigned to the component.

mod contract;
mod key;
mod model;

pub use contract::Component;
#[doc(hidden)]
pub use key::{__with_key_handler_registry, KeyHandlerRegistry};
pub use key::{KeyControl, use_key_event};
pub use model::RenderCtx;
