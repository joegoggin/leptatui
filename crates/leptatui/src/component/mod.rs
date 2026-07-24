//! Component construction support and rendering context.
//!
//! Components render into a [`RenderCtx`], which wraps a Ratatui frame and the
//! rectangular area currently assigned to the component.
//!
//! # Modules
//!
//! - `children` — Component child callback types.
//! - `contract` — Internal focused-control metadata.
//! - `key` — Scoped key-handler registration and dispatch.
//! - `render` — Frame rendering context and target helpers.
//! - `stylesheet` — Scoped component stylesheet registration.

mod children;
mod contract;
mod key;
mod render;
mod stylesheet;

pub use children::{Children, ChildrenFn, ChildrenMut};
#[doc(hidden)]
pub use contract::FocusedControl;
#[doc(hidden)]
pub use key::{__with_key_handler_registry, KeyHandlerRegistry};
pub use key::{KeyControl, use_key_event};
pub(crate) use render::LayoutPhase;
pub use render::RenderCtx;
#[doc(hidden)]
pub use stylesheet::{__register_stylesheet, __with_stylesheet_registry, StylesheetRegistry};
