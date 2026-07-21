//! Parser and expander for the `view!` macro.
//!
//! The view macro accepts a small XML-like syntax and lowers supported terminal
//! elements into Leptatui view builder calls.
//!
//! # Modules
//!
//! - [`attr`] — Element attribute parsing and value classification.
//! - [`child`] — Nested element and text-like child parsing.
//! - [`element`] — Built-in and component element validation and expansion.
//! - [`root`] — Single-root view invocation parsing and expansion.
//! - [`syntax`] — Token lookahead and braced-expression validation.
//! - [`text_content`] — Literal and expression text-content parsing.

mod attr;
mod child;
mod element;
mod root;
mod syntax;
mod text_content;

use proc_macro::TokenStream;

use root::ViewRoot;
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
