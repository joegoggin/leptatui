//! Syntax tree for parsed `stylesheet!` macro input.

mod declaration;
mod rule;
mod selector;
mod stylesheet_root;

pub(super) use stylesheet_root::StylesheetRoot;
