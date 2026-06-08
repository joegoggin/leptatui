//! Syntax tree for parsed `stylesheet!` macro input.
//!
//! This module is split into parser nodes for selectors, declarations, rules,
//! variables, and the top-level stylesheet root.
//!
//! # Modules
//!
//! - [`declaration`] — Style property declaration parsing and expansion.
//! - [`rule`] — Selector plus declaration-block parsing and expansion.
//! - [`selector`] — Stylesheet selector parsing and expansion.
//! - [`stylesheet_root`] — Top-level stylesheet invocation parsing and expansion.
//! - [`value`] — Declaration value parsing for expressions and variables.
//! - [`variable`] — Stylesheet variable definition and lookup support.

mod declaration;
mod rule;
mod selector;
mod stylesheet_root;
mod value;
mod variable;

pub(super) use stylesheet_root::StylesheetRoot;
