//! Code generation for the `component` attribute macro.
//!
//! This module emits the component wrapper type, owner-backed setup, node
//! conversions, default constructor, and render implementation for validated
//! component functions.

use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemFn;

use super::signature;

/// Builds the generated component type for a parsed function.
///
/// # Arguments
///
/// * `input_fn` — Parsed component function to validate and expand.
///
/// # Returns
///
/// A [`TokenStream`] containing the generated component item.
///
/// # Errors
///
/// Returns [`syn::Error`] if the function signature is unsupported.
pub(super) fn component(input_fn: ItemFn) -> syn::Result<TokenStream> {
    signature::validate(&input_fn.sig)?;

    let attrs = input_fn.attrs;
    let vis = input_fn.vis;
    let ident = input_fn.sig.ident;
    let body = input_fn.block;
    let leptatui = crate::utils::crate_path::leptatui();

    let setup_body = quote! {
        {
            let node: #leptatui::Node = (|| #body)().into();
            node
        }
    };

    Ok(quote! {
        #[allow(non_camel_case_types)]
        #(#attrs)*
        #vis struct #ident {
            __leptatui_owner: #leptatui::prelude::Owner,
            __leptatui_node: #leptatui::Node,
        }

        impl #ident {
            #[doc = "Creates a component value."]
            #vis fn new() -> Self {
                let __leptatui_owner = #leptatui::prelude::Owner::new();
                let __leptatui_node = __leptatui_owner.with(Self::__setup_tree);

                Self {
                    __leptatui_owner,
                    __leptatui_node,
                }
            }

            #[doc(hidden)]
            fn __setup_tree() -> #leptatui::Node {
                #setup_body
            }

            #[doc = "Converts this component into a Leptatui node."]
            #vis fn into_node(self) -> #leptatui::Node {
                #leptatui::component(self)
            }
        }

        impl ::core::default::Default for #ident {
            #[doc = "Creates the default component value."]
            fn default() -> Self {
                Self::new()
            }
        }

        impl ::core::convert::From<#ident> for #leptatui::Node {
            #[doc = "Converts the component into a Leptatui node."]
            fn from(component: #ident) -> Self {
                component.into_node()
            }
        }

        impl #leptatui::Component for #ident {
            #[doc = "Renders the component into the provided Leptatui context."]
            fn render(
                &mut self,
                ctx: &mut #leptatui::RenderCtx<'_, '_>,
            ) -> #leptatui::Result<()> {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_node = &self.__leptatui_node;

                __leptatui_owner.with(|| {
                    #leptatui::context::__with_context_scope_if_missing(|| {
                        ctx.render_node(__leptatui_node)
                    })
                })
            }

            #[doc = "Dispatches events through the component's rendered node tree."]
            fn handle_event(
                &mut self,
                event: #leptatui::__private::Event,
            ) -> #leptatui::Result<#leptatui::AppControl> {
                let __leptatui_owner = &self.__leptatui_owner;
                let __leptatui_node = &mut self.__leptatui_node;

                __leptatui_owner.with(|| {
                    __leptatui_node.handle_event(event)
                })
            }
        }
    })
}
