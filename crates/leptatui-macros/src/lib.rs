//! Internal proc-macro crate for Leptatui.
//!
//! This crate contains procedural macros that support the public `leptatui`
//! runtime crate.
//!
//! # Modules
//!
//! - `component` — Component attribute parsing and code generation.
//! - `crate_path` — Runtime-crate path resolution for generated code.
//! - `route_params` — Typed route and query parameter derives.
//! - `stylesheet` — Stylesheet syntax parsing and expansion.
//! - `view` — Declarative view syntax parsing and expansion.

mod component;
mod crate_path;
mod route_params;
mod stylesheet;
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

/// Implements typed conversion from matched route parameters.
///
/// # Arguments
///
/// * `input` — Named, non-generic struct receiving the implementation.
///
/// # Returns
///
/// A [`TokenStream`] containing the generated implementation or diagnostic.
#[proc_macro_derive(RouteParams, attributes(param))]
pub fn derive_route_params(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    route_params::expand(input, route_params::ParameterSource::Route)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Implements typed parsing and serialization for query parameters.
///
/// # Arguments
///
/// * `input` — Named, non-generic struct receiving the implementation.
///
/// # Returns
///
/// A [`TokenStream`] containing the generated implementation or diagnostic.
#[proc_macro_derive(QueryParams, attributes(param))]
pub fn derive_query_params(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    route_params::expand(input, route_params::ParameterSource::Query)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
