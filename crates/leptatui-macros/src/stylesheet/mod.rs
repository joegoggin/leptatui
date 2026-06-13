//! Parser and expander for the `stylesheet!` macro.
//!
//! The stylesheet macro accepts flat terminal selector rules, lowers them into
//! Leptatui stylesheet builder calls, and registers them when invoked during
//! generated component setup.

mod model;

use proc_macro::TokenStream;

use model::StylesheetRoot;
use syn::Error;

/// Expands `stylesheet!` input into a registered Leptatui [`Stylesheet`]
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
