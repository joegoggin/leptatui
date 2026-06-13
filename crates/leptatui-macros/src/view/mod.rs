//! Parser and expander for the `view!` macro.
//!
//! The view macro accepts a small XML-like syntax and lowers supported terminal
//! elements into Leptatui view builder calls.

mod model;
mod utils;

use proc_macro::TokenStream;

use model::ViewRoot;
use syn::Error;

/// Expands `view!` input into Leptatui view builder calls.
///
/// # Arguments
///
/// * `input` — Token stream passed to the `view!` macro invocation.
///
/// # Returns
///
/// A [`TokenStream`] containing generated view code or compile errors.
pub(crate) fn expand(input: TokenStream) -> TokenStream {
    syn::parse::<ViewRoot>(input)
        .and_then(ViewRoot::expand)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
