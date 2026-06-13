//! Internal proc-macro crate for Leptatui.
//!
//! This crate contains procedural macros that support the public `leptatui`
//! runtime crate.

mod component;
mod stylesheet;
mod utils;
mod view;

use proc_macro::TokenStream;

/// Converts a zero-argument function returning a view-compatible value into a
/// Leptatui component type.
#[proc_macro_attribute]
pub fn component(args: TokenStream, input: TokenStream) -> TokenStream {
    component::expand(args, input)
}

/// Converts declarative terminal element syntax into Leptatui view builders.
#[proc_macro]
pub fn view(input: TokenStream) -> TokenStream {
    view::expand(input)
}

/// Converts declarative terminal stylesheet syntax into Leptatui style rules.
#[proc_macro]
pub fn stylesheet(input: TokenStream) -> TokenStream {
    stylesheet::expand(input)
}
