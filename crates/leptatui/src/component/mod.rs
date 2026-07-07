//! Component rendering contract.
//!
//! Components render into a [`RenderCtx`], which wraps a Ratatui frame and the
//! rectangular area currently assigned to the component.

mod children;
mod contract;
mod key;
mod model;
mod stylesheet;

pub use children::{Children, ChildrenFn, ChildrenMut};
pub use contract::Component;
#[doc(hidden)]
pub use contract::FocusedControl;
#[doc(hidden)]
pub use key::{__with_key_handler_registry, KeyHandlerRegistry};
pub use key::{KeyControl, use_key_event};
pub use model::RenderCtx;
#[doc(hidden)]
pub use stylesheet::{__register_stylesheet, __with_stylesheet_registry, StylesheetRegistry};
