//! Expansion support for the `component` attribute macro.
//!
//! This module validates component function signatures and emits the component
//! type, node conversion, and render implementation used by the runtime crate.

mod expand;
mod signature;

use proc_macro::TokenStream;

use syn::{Error, ItemFn, parse_macro_input};

/// Expands a `#[component]` function into a Leptatui component type.
///
/// # Arguments
///
/// * `args` — Attribute arguments supplied to `#[component]`.
/// * `input` — Function item annotated with `#[component]`.
///
/// # Returns
///
/// A [`TokenStream`] containing generated component code or compile errors.
pub(crate) fn expand(args: TokenStream, input: TokenStream) -> TokenStream {
    if !args.is_empty() {
        return Error::new(
            proc_macro2::Span::call_site(),
            "#[component] does not accept arguments",
        )
        .to_compile_error()
        .into();
    }

    let input_fn = parse_macro_input!(input as ItemFn);

    expand::component(input_fn)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
