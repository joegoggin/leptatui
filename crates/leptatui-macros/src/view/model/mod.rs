//! Syntax tree for parsed `view!` macro input.
//!
//! Each model in this module keeps its parsing and expansion impls next to the
//! type declaration so the model's behavior is easy to audit in one place.

mod attr;
mod child;
mod element;
mod text_content;
mod view_root;

pub(super) use view_root::ViewRoot;
