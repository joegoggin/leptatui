//! Internal proc-macro crate for Leptatui.
//!
//! This crate contains procedural macros that support the public `leptatui`
//! runtime crate.

mod component;
mod utils;
mod view;

use proc_macro::TokenStream;

/// Converts a zero-argument function returning a node-compatible value into a
/// Leptatui component type.
#[proc_macro_attribute]
pub fn component(args: TokenStream, input: TokenStream) -> TokenStream {
    component::expand(args, input)
}

/// Converts declarative terminal element syntax into Leptatui node builders.
#[proc_macro]
pub fn view(input: TokenStream) -> TokenStream {
    view::expand(input)
}
