use proc_macro::TokenStream;

use quote::quote;
use syn::{Error, ItemFn, ReturnType, Signature, Type, parse_macro_input};

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

fn expand_component(input_fn: ItemFn) -> syn::Result<proc_macro2::TokenStream> {
    validate_signature(&input_fn.sig)?;

    let attrs = input_fn.attrs;
    let vis = input_fn.vis;
    let ident = input_fn.sig.ident;
    let body = input_fn.block;

    Ok(quote! {
        #[allow(non_camel_case_types)]
        #(#attrs)*
        #vis struct #ident;

        impl #ident {
            #vis const fn new() -> Self {
                Self
            }
        }

        impl ::core::default::Default for #ident {
            fn default() -> Self {
                Self::new()
            }
        }

        impl ::leptatui::Component for #ident {
            fn render(
                &mut self,
                ctx: &mut ::leptatui::RenderCtx<'_, '_>,
            ) -> ::leptatui::Result<()> {
                let node: ::leptatui::Node = (|| #body)().into();
                ctx.render_node(&node)
            }
        }
    })
}

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

fn is_unit_type(ty: &Type) -> bool {
    matches!(ty, Type::Tuple(tuple) if tuple.elems.is_empty())
}
