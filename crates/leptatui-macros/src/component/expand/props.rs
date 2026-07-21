//! Generated component props and type-state builders.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Visibility};

use crate::component::signature::Prop;

/// Expands the generated props struct and typed builder for prop components.
pub(super) fn expand_props_api(vis: &Visibility, component: &Ident, props: &[Prop]) -> TokenStream {
    if props.is_empty() {
        return TokenStream::new();
    }

    let props_ident = props_ident(component);
    let builder_ident = builder_ident(component);
    let missing_ident = missing_ident(component);
    let field_defs = props.iter().map(|prop| {
        let attrs = &prop.attrs;
        let ident = &prop.ident;
        let ty = &prop.ty;

        quote! {
            #(#attrs)*
            #vis #ident: #ty
        }
    });

    let state_idents = props
        .iter()
        .map(|prop| prop.state_ident(component))
        .collect::<Vec<_>>();
    let initial_state_types = props.iter().map(|prop| match &prop.default {
        Some(_) => {
            let ty = &prop.ty;
            quote! { #ty }
        }
        None => quote! { #missing_ident },
    });
    let initial_values = props.iter().map(|prop| {
        let ident = &prop.ident;
        let value = match &prop.default {
            Some(default) => default.initial_value(prop.ty.as_ref()),
            None => quote! { #missing_ident },
        };

        quote! { #ident: #value }
    });
    let builder_fields = props.iter().zip(&state_idents).map(|(prop, state)| {
        let ident = &prop.ident;
        quote! { #ident: #state }
    });
    let setters = props
        .iter()
        .enumerate()
        .map(|(index, prop)| expand_prop_setter(vis, component, props, index, prop));
    let build_args = props.iter().map(|prop| {
        let ty = &prop.ty;
        quote! { #ty }
    });
    let build_fields = props.iter().map(|prop| {
        let ident = &prop.ident;
        quote! { #ident: self.#ident }
    });

    quote! {
        #[doc = "Props for the generated component."]
        #vis struct #props_ident {
            #(#field_defs,)*
        }

        #[doc = "Builder for generated component props."]
        #vis struct #builder_ident<#(#state_idents),*> {
            #(#builder_fields,)*
        }

        #[doc(hidden)]
        #vis struct #missing_ident;

        impl #props_ident {
            #[doc = "Creates a builder for component props."]
            #vis fn builder() -> #builder_ident<#(#initial_state_types),*> {
                #builder_ident {
                    #(#initial_values,)*
                }
            }
        }

        #(#setters)*

        impl #builder_ident<#(#build_args),*> {
            #[doc = "Builds component props."]
            #vis fn build(self) -> #props_ident {
                #props_ident {
                    #(#build_fields,)*
                }
            }
        }
    }
}

/// Expands one setter method for the generated props builder.
fn expand_prop_setter(
    vis: &Visibility,
    component: &Ident,
    props: &[Prop],
    index: usize,
    prop: &Prop,
) -> TokenStream {
    let builder_ident = builder_ident(component);
    let missing_ident = missing_ident(component);
    let state_idents = props
        .iter()
        .map(|prop| prop.state_ident(component))
        .collect::<Vec<_>>();
    let ident = &prop.ident;
    let ty = &prop.ty;
    let setter_ty = if prop.into {
        quote! { impl ::core::convert::Into<#ty> }
    } else {
        quote! { #ty }
    };
    let setter_value = if prop.into {
        quote! { ::core::convert::Into::into(#ident) }
    } else {
        quote! { #ident }
    };
    let impl_generics = state_idents
        .iter()
        .enumerate()
        .filter_map(|(arg_index, state)| {
            (arg_index != index || prop.default.is_some()).then_some(state)
        })
        .collect::<Vec<_>>();
    let impl_generics = if impl_generics.is_empty() {
        TokenStream::new()
    } else {
        quote! { <#(#impl_generics),*> }
    };
    let impl_args = state_idents.iter().enumerate().map(|(arg_index, state)| {
        if arg_index == index && prop.default.is_none() {
            quote! { #missing_ident }
        } else {
            quote! { #state }
        }
    });
    let return_args = state_idents.iter().enumerate().map(|(arg_index, state)| {
        if arg_index == index {
            quote! { #ty }
        } else {
            quote! { #state }
        }
    });
    let fields = props.iter().enumerate().map(|(field_index, field)| {
        let field_ident = &field.ident;
        if field_index == index {
            quote! { #field_ident: #setter_value }
        } else {
            quote! { #field_ident: self.#field_ident }
        }
    });

    quote! {
        impl #impl_generics #builder_ident<#(#impl_args),*> {
            #[doc = "Sets a component prop value."]
            #vis fn #ident(self, #ident: #setter_ty) -> #builder_ident<#(#return_args),*> {
                #builder_ident {
                    #(#fields,)*
                }
            }
        }
    }
}

/// Returns the generated props struct identifier.
pub(super) fn props_ident(component: &Ident) -> Ident {
    format_ident!("{component}Props")
}

/// Returns the generated props builder identifier.
fn builder_ident(component: &Ident) -> Ident {
    format_ident!("{component}PropsBuilder")
}

/// Returns the generated missing-prop marker identifier.
fn missing_ident(component: &Ident) -> Ident {
    format_ident!("{component}PropsMissing")
}
