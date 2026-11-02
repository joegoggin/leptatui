//! Generated component constructors and setup functions.

use std::{env, path::Path};

use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::{Ident, LitStr, Visibility};

use crate::component::signature::{FallibleError, Prop};

use super::props::props_ident;

/// Expands generated constructors for a component.
pub(super) fn expand_constructors(
    vis: &Visibility,
    component: &Ident,
    props: &[Prop],
) -> TokenStream {
    if props.is_empty() {
        return quote! {
            #[doc = "Creates a component value."]
            #vis fn new() -> Self {
                Self::__create(Self::__setup_tree)
            }
        };
    }

    let props_ident = props_ident(component);
    let default_new = if props_can_default(props) {
        quote! {
            #[doc = "Creates a component value with default props."]
            #vis fn new() -> Self {
                Self::with_props(#props_ident::builder().build())
            }
        }
    } else {
        TokenStream::new()
    };

    quote! {
        #default_new

        #[doc = "Creates a component value with explicit props."]
        #vis fn with_props(__leptatui_props: #props_ident) -> Self {
            Self::__create(move || Self::__setup_tree(__leptatui_props))
        }
    }
}

/// Expands the hidden setup function that runs the original component body.
pub(super) fn expand_setup_fn(
    component: &Ident,
    props: &[Prop],
    body: TokenStream,
    fallible_error: Option<&FallibleError>,
    leptatui: &TokenStream,
) -> TokenStream {
    let setup_body = if let Some(error) = fallible_error {
        let source_file = LitStr::new(&component_source_file(component), component.span());
        let source_line = Literal::usize_unsuffixed(component.span().start().line);
        let error = match error {
            FallibleError::ViewError => quote! { #leptatui::ViewError },
            FallibleError::Explicit(error) => quote! { #error },
        };
        quote! {
            {
                let __leptatui_result:
                    ::core::result::Result<#leptatui::AnyView, #error> =
                    (|| #body)();
                match __leptatui_result {
                    ::core::result::Result::Ok(view) => view,
                    ::core::result::Result::Err(error) => {
                        let error: #leptatui::ViewError =
                            ::core::convert::Into::into(error);
                        #leptatui::__private::__view_error(
                            error,
                            #source_file,
                            #source_line,
                        )
                    }
                }
            }
        }
    } else {
        quote! {
            {
                let view: #leptatui::AnyView =
                    #leptatui::IntoView::into_view((|| #body)());
                view
            }
        }
    };

    if props.is_empty() {
        return quote! {
            #[doc(hidden)]
            fn __setup_tree() -> #leptatui::AnyView {
                #setup_body
            }
        };
    }

    let props_ident = props_ident(component);
    let field_names = props.iter().map(|prop| &prop.ident);

    quote! {
        #[doc(hidden)]
        fn __setup_tree(__leptatui_props: #props_ident) -> #leptatui::AnyView {
            let #props_ident {
                #(#field_names,)*
            } = __leptatui_props;

            #setup_body
        }
    }
}

/// Returns the source-relative display path for a component declaration.
///
/// The compiler can omit the display filename for tokens originating in an
/// included file, so this falls back to the span's local path relative to the
/// caller manifest or compiler working directory.
///
/// # Arguments
///
/// * `component` — Component identifier whose declaration path is required.
///
/// # Returns
///
/// A [`String`] containing a source-relative path or an unknown-path marker.
fn component_source_file(component: &Ident) -> String {
    let span = component.span();
    let display = span.file();
    if !display.is_empty() && display != "<token stream>" && Path::new(&display).is_relative() {
        return display;
    }

    let Some(local) = span.local_file() else {
        return "<unknown>".to_owned();
    };
    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR");
    let current_dir = env::current_dir().ok();

    manifest_dir
        .as_deref()
        .map(Path::new)
        .and_then(|root| local.strip_prefix(root).ok())
        .or_else(|| {
            current_dir
                .as_deref()
                .and_then(|root| local.strip_prefix(root).ok())
        })
        .or_else(|| local.file_name().map(Path::new))
        .unwrap_or(Path::new("<unknown>"))
        .to_string_lossy()
        .into_owned()
}

/// Expands `Default` when the generated component can be created without props.
pub(super) fn expand_default_impl(component: &Ident, props: &[Prop]) -> TokenStream {
    if !props_can_default(props) {
        return TokenStream::new();
    }

    quote! {
        impl ::core::default::Default for #component {
            #[doc = "Creates the default component value."]
            fn default() -> Self {
                Self::new()
            }
        }
    }
}

/// Returns whether every prop can be omitted.
fn props_can_default(props: &[Prop]) -> bool {
    props.iter().all(|prop| prop.default.is_some())
}
