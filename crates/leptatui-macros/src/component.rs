//! Expansion support for the `component` attribute macro.
//!
//! This module validates component function signatures and emits the component
//! type, node conversion, and render implementation used by the runtime crate.

use proc_macro::TokenStream;

use quote::quote;
use syn::{Error, ItemFn, ReturnType, Signature, Type, parse_macro_input};

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

    expand_component(input_fn)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

/// Builds the generated component type for a parsed function.
///
/// # Arguments
///
/// * `input_fn` — Parsed component function to validate and expand.
///
/// # Returns
///
/// A [`proc_macro2::TokenStream`] containing the generated component item.
///
/// # Errors
///
/// Returns [`syn::Error`] if the function signature is unsupported.
fn expand_component(input_fn: ItemFn) -> syn::Result<proc_macro2::TokenStream> {
    validate_signature(&input_fn.sig)?;

    let attrs = input_fn.attrs;
    let vis = input_fn.vis;
    let ident = input_fn.sig.ident;
    let body = input_fn.block;

    let render_body = quote! {
        ::leptatui::context::__with_context_scope(|| {
            let node: ::leptatui::Node = (|| #body)().into();
            node
        })
    };

    Ok(quote! {
        #[allow(non_camel_case_types)]
        #(#attrs)*
        #vis struct #ident;

        impl #ident {
            #[doc = "Creates a component value."]
            #vis const fn new() -> Self {
                Self
            }

            #[doc = "Converts this component into a Leptatui node."]
            #vis fn into_node(self) -> ::leptatui::Node {
                #render_body
            }
        }

        impl ::core::default::Default for #ident {
            #[doc = "Creates the default component value."]
            fn default() -> Self {
                Self::new()
            }
        }

        impl ::core::convert::From<#ident> for ::leptatui::Node {
            #[doc = "Converts the component into a Leptatui node."]
            fn from(component: #ident) -> Self {
                component.into_node()
            }
        }

        impl ::leptatui::Component for #ident {
            #[doc = "Renders the component into the provided Leptatui context."]
            fn render(
                &mut self,
                ctx: &mut ::leptatui::RenderCtx<'_, '_>,
            ) -> ::leptatui::Result<()> {
                let node = Self::new().into_node();
                ctx.render_node(&node)
            }
        }
    })
}

/// Validates that a component function can be expanded.
///
/// # Arguments
///
/// * `sig` — Function signature from the annotated component function.
///
/// # Returns
///
/// An empty [`syn::Result`] when the signature is supported.
///
/// # Errors
///
/// Returns [`syn::Error`] if the function is const, async, unsafe, extern,
/// generic, parameterized, missing a return type, or returning `()`.
fn validate_signature(sig: &Signature) -> syn::Result<()> {
    if let Some(constness) = &sig.constness {
        return Err(Error::new_spanned(
            constness,
            "#[component] functions cannot be const",
        ));
    }

    if let Some(asyncness) = &sig.asyncness {
        return Err(Error::new_spanned(
            asyncness,
            "#[component] functions cannot be async",
        ));
    }

    if let Some(unsafety) = &sig.unsafety {
        return Err(Error::new_spanned(
            unsafety,
            "#[component] functions cannot be unsafe",
        ));
    }

    if let Some(abi) = &sig.abi {
        return Err(Error::new_spanned(
            abi,
            "#[component] functions cannot use an extern ABI",
        ));
    }

    if !sig.generics.params.is_empty() || sig.generics.where_clause.is_some() {
        return Err(Error::new_spanned(
            &sig.generics,
            "#[component] functions cannot be generic yet",
        ));
    }

    if !sig.inputs.is_empty() {
        return Err(Error::new_spanned(
            &sig.inputs,
            "#[component] functions cannot take parameters yet",
        ));
    }

    match &sig.output {
        ReturnType::Default => Err(Error::new_spanned(
            &sig.ident,
            "#[component] functions must return a value convertible into leptatui::Node",
        )),
        ReturnType::Type(_, ty) if is_unit_type(ty) => Err(Error::new_spanned(
            ty,
            "#[component] functions must not return ()",
        )),
        ReturnType::Type(_, _) => Ok(()),
    }
}

/// Returns whether a type is the unit type.
///
/// # Arguments
///
/// * `ty` — Parsed Rust type to inspect.
///
/// # Returns
///
/// A [`bool`] indicating whether `ty` is `()`.
fn is_unit_type(ty: &Type) -> bool {
    matches!(ty, Type::Tuple(tuple) if tuple.elems.is_empty())
}
