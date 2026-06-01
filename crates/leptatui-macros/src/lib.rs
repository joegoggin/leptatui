//! Internal proc-macro crate for Leptatui.
//!
//! This crate contains procedural macros that support the public `leptatui`
//! runtime crate.

mod component;

use proc_macro::TokenStream;

/// Converts a zero-argument function returning a node-compatible value into a
/// Leptatui component type.
#[proc_macro_attribute]
pub fn component(args: TokenStream, input: TokenStream) -> TokenStream {
    component::expand(args, input)
}
