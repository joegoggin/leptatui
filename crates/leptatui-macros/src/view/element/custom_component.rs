//! Expansion of PascalCase application component tags.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Error, Result};

use super::Element;

impl Element {
    /// Expands a PascalCase component tag into a component constructor call.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing a view expression for the component.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if explicit `children` and nested children are
    /// both supplied.
    pub(super) fn expand_component(&self) -> Result<TokenStream> {
        if let Some(attr) = self
            .attrs
            .iter()
            .find(|attr| attr.name == "children" && !self.children.is_empty())
        {
            return Err(Error::new_spanned(
                &attr.name,
                "view! component cannot specify a children prop and child content",
            ));
        }

        let leptatui = crate::crate_path::leptatui();
        let name = &self.name;
        let preserve_on_reconcile = if self.attrs.is_empty() && self.children.is_empty() {
            quote! { true }
        } else {
            quote! { false }
        };
        let component = if self.attrs.is_empty() && self.children.is_empty() {
            quote! { #name::new() }
        } else {
            let props = format_ident!("{name}Props");
            let attr_bindings = self.attrs.iter().enumerate().map(|(index, attr)| {
                let binding = format_ident!("__leptatui_prop_{index}");
                let value = attr.value.to_tokens();

                quote! { let #binding = #value; }
            });
            let attr_setters = self.attrs.iter().enumerate().map(|(index, attr)| {
                let name = &attr.name;
                let binding = format_ident!("__leptatui_prop_{index}");

                quote! { .#name(#binding) }
            });
            let children = if self.children.is_empty() {
                TokenStream::new()
            } else {
                let children = self.expand_component_children()?;
                let children = Self::flatten_child_values(&children);
                quote! {
                    .children(::std::boxed::Box::new(move || #children))
                }
            };

            return Ok(quote! {{
                #(#attr_bindings)*
                #leptatui::__private::__component_factory(
                    #preserve_on_reconcile,
                    move || {
                        #name::with_props(
                            #props::builder()
                                #(#attr_setters)*
                                #children
                                .build()
                        )
                    },
                )
            }});
        };

        Ok(quote! {
            #leptatui::__private::__component_factory(#preserve_on_reconcile, move || #component)
        })
    }
}
