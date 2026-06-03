//! Code generation for the `component` attribute macro.
//!
//! This module emits the component wrapper type, node conversions, default
//! constructor, and render implementation for validated component functions.

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

    let render_body = quote! {
        {
            let node: #leptatui::Node = (|| #body)().into();
            node
        }
    };

    Ok(quote! {
        #[allow(non_camel_case_types)]
        #(#attrs)*
        #vis struct #ident {
            __leptatui_node: ::core::option::Option<#leptatui::Node>,
        }

        impl #ident {
            #[doc = "Creates a component value."]
            #vis const fn new() -> Self {
                Self {
                    __leptatui_node: ::core::option::Option::None,
                }
            }

            #[doc(hidden)]
            fn __render_tree() -> #leptatui::Node {
                #render_body
            }

            #[doc(hidden)]
            fn __rerendered_node(&mut self) -> &mut #leptatui::Node {
                let mut node = Self::__render_tree();
                if let ::core::option::Option::Some(previous) = &self.__leptatui_node {
                    #leptatui::__private::__reconcile_node(&mut node, previous);
                }

                self.__leptatui_node = ::core::option::Option::Some(node);
                self.__leptatui_node
                    .as_mut()
                    .expect("generated component render tree should be initialized")
            }

            #[doc(hidden)]
            fn __event_node(&mut self) -> &mut #leptatui::Node {
                self.__leptatui_node.get_or_insert_with(Self::__render_tree)
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
                #leptatui::context::__with_context_scope_if_missing(|| {
                    ctx.render_node(self.__rerendered_node())
                })
            }

            #[doc = "Dispatches events through the component's rendered node tree."]
            fn handle_event(
                &mut self,
                event: #leptatui::__private::Event,
            ) -> #leptatui::Result<#leptatui::AppControl> {
                self.__event_node().handle_event(event)
            }
        }
    })
}
