//! Generated component constructors and setup functions.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Visibility};

use crate::component::signature::Prop;

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
    leptatui: &TokenStream,
) -> TokenStream {
    let setup_body = quote! {
        {
            let view: #leptatui::AnyView = #leptatui::IntoView::into_view((|| #body)());
            view
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
