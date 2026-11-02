//! Derive expansion for typed route and query parameter models.

use std::collections::BTreeSet;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Data, DeriveInput, Error, Fields, GenericArgument, Ident, LitStr, PathArguments, Result, Type,
    ext::IdentExt, parse_quote,
};

/// Parameter source implemented by one typed model.
#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum ParameterSource {
    /// Path parameters captured by a matched route.
    Route,
    /// Query parameters parsed from the current location.
    Query,
}

impl ParameterSource {
    /// Returns the implemented runtime trait identifier.
    ///
    /// # Returns
    ///
    /// An [`Ident`] naming `RouteParams` or `QueryParams`.
    fn trait_ident(self) -> Ident {
        match self {
            Self::Route => parse_quote!(RouteParams),
            Self::Query => parse_quote!(QueryParams),
        }
    }

    /// Returns the implemented conversion method identifier.
    ///
    /// # Returns
    ///
    /// An [`Ident`] naming the source-specific conversion method.
    fn method_ident(self) -> Ident {
        match self {
            Self::Route => parse_quote!(from_params),
            Self::Query => parse_quote!(from_query),
        }
    }
}

/// Expands a typed route or query parameter derive.
///
/// # Arguments
///
/// * `input` — Parsed derive input containing one named, non-generic struct.
/// * `source` — Parameter source selecting the implemented runtime trait.
///
/// # Returns
///
/// A [`TokenStream`] implementing the requested parameter trait.
///
/// # Errors
///
/// Returns [`syn::Error`] if the input is not a named, non-generic struct or
/// contains invalid or duplicate parameter mappings.
pub(crate) fn expand(input: DeriveInput, source: ParameterSource) -> Result<TokenStream> {
    if !input.generics.params.is_empty() || input.generics.where_clause.is_some() {
        return Err(Error::new_spanned(
            &input.generics,
            "typed parameter derives do not support generic structs",
        ));
    }
    let Data::Struct(data) = &input.data else {
        return Err(Error::new_spanned(
            &input.ident,
            "typed parameter derives require a struct with named fields",
        ));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(Error::new_spanned(
            &data.fields,
            "typed parameter derives require a struct with named fields",
        ));
    };

    let leptatui = crate::crate_path::leptatui();
    let trait_ident = source.trait_ident();
    let method_ident = source.method_ident();
    let name = &input.ident;
    let mut mapped_names = BTreeSet::new();
    let mut initializers = Vec::new();
    let mut serializers = Vec::new();

    for field in &fields.named {
        let ident = field
            .ident
            .as_ref()
            .expect("named fields always contain identifiers");
        let parameter_name = parameter_name(field, ident)?;
        if !mapped_names.insert(parameter_name.value()) {
            return Err(Error::new_spanned(
                field,
                format!(
                    "duplicate typed parameter mapping `{}`",
                    parameter_name.value()
                ),
            ));
        }
        let initializer = if let Some(inner) = option_inner(&field.ty) {
            if source == ParameterSource::Query {
                serializers.push(quote! {
                    if let ::core::option::Option::Some(value) = &self.#ident {
                        #leptatui::__private::__push_query_param(
                            &mut query,
                            #parameter_name,
                            value,
                        );
                    }
                });
            }
            quote! {
                #ident: #leptatui::__private::__optional_param::<#inner>(params, #parameter_name)?
            }
        } else {
            if source == ParameterSource::Query {
                serializers.push(quote! {
                    #leptatui::__private::__push_query_param(
                        &mut query,
                        #parameter_name,
                        &self.#ident,
                    );
                });
            }
            let ty = &field.ty;
            quote! {
                #ident: #leptatui::__private::__required_param::<#ty>(params, #parameter_name)?
            }
        };
        initializers.push(initializer);
    }

    let serializer = if source == ParameterSource::Query {
        quote! {
            fn to_query_string(&self) -> ::std::string::String {
                let mut query = ::std::string::String::new();
                #(#serializers)*
                query
            }
        }
    } else {
        TokenStream::new()
    };

    Ok(quote! {
        impl #leptatui::#trait_ident for #name {
            fn #method_ident(
                params: &#leptatui::ParamsMap,
            ) -> ::core::result::Result<Self, #leptatui::ParamsError> {
                ::core::result::Result::Ok(Self {
                    #(#initializers),*
                })
            }

            #serializer
        }
    })
}

/// Returns the mapped parameter name for one struct field.
///
/// # Arguments
///
/// * `field` — Named struct field whose attributes are inspected.
/// * `ident` — Rust field identifier used as the default mapping.
///
/// # Returns
///
/// A [`LitStr`] containing the exact route or query parameter name.
///
/// # Errors
///
/// Returns [`syn::Error`] if a `param` attribute is malformed, unsupported, or
/// defines `name` more than once.
fn parameter_name(field: &syn::Field, ident: &Ident) -> Result<LitStr> {
    let mut renamed = None;
    for attribute in field
        .attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident("param"))
    {
        attribute.parse_nested_meta(|meta| {
            if !meta.path.is_ident("name") {
                return Err(meta.error("unsupported param attribute; expected name = \"...\""));
            }
            if renamed.is_some() {
                return Err(meta.error("param name can only be specified once"));
            }
            renamed = Some(meta.value()?.parse::<LitStr>()?);
            Ok(())
        })?;
    }

    Ok(renamed.unwrap_or_else(|| LitStr::new(&ident.unraw().to_string(), ident.span())))
}

/// Returns the inner type when a field uses `Option<T>`.
///
/// # Arguments
///
/// * `ty` — Field type inspected for an `Option` path.
///
/// # Returns
///
/// An optional [`Type`] reference containing the option's value type.
fn option_inner(ty: &Type) -> Option<&Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    let segment = path.path.segments.last()?;
    if segment.ident != "Option" {
        return None;
    }
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    let [GenericArgument::Type(inner)] = arguments.args.iter().collect::<Vec<_>>().as_slice()
    else {
        return None;
    };
    Some(inner)
}
