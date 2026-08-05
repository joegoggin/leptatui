//! Parser and expander for the `stylesheet!` macro.
//!
//! The stylesheet macro accepts terminal selector rules, including nested
//! descendants, parent pseudo-classes, BEM class suffixes, and selector-local
//! media queries. It lowers them into Leptatui stylesheet builder calls and
//! registers them when invoked during generated component setup.
//!
//! # Modules
//!
//! - [`declaration`] — Property declaration parsing and expansion.
//! - [`import`] — Imported stylesheet module bindings.
//! - [`media`] — Responsive media-query parsing and expansion.
//! - [`mixin`] — Mixin definitions, includes, and cycle detection.
//! - [`root`] — Complete stylesheet invocation parsing and expansion.
//! - [`rule`] — Selector rule parsing and media-aware expansion.
//! - [`selector`] — Type, class, id, pseudo-class, descendant, and BEM selectors.
//! - [`value`] — Literal, local-variable, and imported-variable values.
//! - [`variable`] — Stylesheet variable definitions and references.

mod declaration;
mod import;
mod media;
mod mixin;
mod root;
mod rule;
mod selector;
mod value;
mod variable;

use proc_macro::TokenStream;

use root::StylesheetRoot;
use syn::Error;

/// Expands `stylesheet!` input into a registered Leptatui `Stylesheet`
/// expression.
///
/// # Arguments
///
/// * `input` — Token stream passed to the `stylesheet!` macro invocation.
///
/// # Returns
///
/// A [`TokenStream`] containing generated stylesheet code or compile errors.
pub(crate) fn expand(input: TokenStream) -> TokenStream {
    syn::parse::<StylesheetRoot>(input)
        .and_then(StylesheetRoot::expand)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
