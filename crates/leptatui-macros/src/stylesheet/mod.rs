//! Parser and expander for the `stylesheet!` macro.
//!
//! The stylesheet macro accepts flat terminal selector rules, lowers them into
//! Leptatui stylesheet builder calls, and registers them when invoked during
//! generated component setup.
//!
//! # Modules
//!
//! Each syntax concept has a direct parser module; [`root`] coordinates parsing
//! and expansion for the complete invocation.

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
