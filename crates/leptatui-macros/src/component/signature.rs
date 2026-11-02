//! Signature analysis for `#[component]` functions.
//!
//! This module rejects unsupported component function shapes and extracts
//! Leptos-style prop metadata from supported function parameters.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Attribute, Error, Expr, FnArg, GenericArgument, Ident, Pat, PatType, PathArguments, ReturnType,
    Signature, Type, parse::ParseStream,
};

/// Error type emitted by a supported fallible component signature.
pub(super) enum FallibleError {
    /// Leptatui's type-erased [`ViewResult`](leptatui::ViewResult) error.
    ViewError,
    /// Explicit error type from a two-parameter [`Result`](std::result::Result).
    Explicit(Box<Type>),
}

/// Metadata extracted from a supported component prop parameter.
pub(super) struct Prop {
    /// Attributes copied to the generated props struct field.
    pub(super) attrs: Vec<Attribute>,
    /// Prop field and local variable name.
    pub(super) ident: Ident,
    /// Prop value type.
    pub(super) ty: Box<Type>,
    /// Defaulting behavior for omitted props.
    pub(super) default: Option<PropDefault>,
    /// Whether setter inputs should be converted with [`Into`].
    pub(super) into: bool,
}

impl Prop {
    /// Returns the type-state parameter name for this prop in the generated builder.
    pub(super) fn state_ident(&self, component: &Ident) -> Ident {
        let prop = to_pascal_case(&self.ident.to_string());
        format_ident!("{component}Props{prop}State")
    }
}

/// Defaulting behavior accepted by `#[prop(...)]`.
pub(super) enum PropDefault {
    /// `#[prop(optional)]`.
    Optional,
    /// `#[prop(default = expr)]`.
    Expr(Box<Expr>),
}

impl PropDefault {
    /// Expands the initial builder value for this default.
    pub(super) fn initial_value(&self, ty: &Type) -> TokenStream {
        match self {
            Self::Optional => quote! { <#ty as ::core::default::Default>::default() },
            Self::Expr(expr) => quote! { #expr },
        }
    }
}

/// Validates the component signature and extracts prop metadata.
///
/// # Arguments
///
/// * `sig` — Function signature from the annotated component function.
///
/// # Returns
///
/// Supported props in source order.
///
/// # Errors
///
/// Returns [`syn::Error`] if the function is const, async, unsafe, extern,
/// generic, missing a return type, returning `()`, or has unsupported props.
pub(super) fn analyze(sig: &Signature) -> syn::Result<Vec<Prop>> {
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

    match &sig.output {
        ReturnType::Default => {
            return Err(Error::new_spanned(
                &sig.ident,
                "#[component] functions must return a value implementing leptatui::IntoView",
            ));
        }
        ReturnType::Type(_, ty) if is_unit_type(ty) => {
            return Err(Error::new_spanned(
                ty,
                "#[component] functions must not return ()",
            ));
        }
        ReturnType::Type(_, _) => {}
    }

    sig.inputs.iter().map(parse_prop).collect()
}

/// Returns the error model declared by a fallible component signature.
///
/// # Arguments
///
/// * `sig` — Component function signature to inspect.
///
/// # Returns
///
/// An optional [`FallibleError`] for `ViewResult<T>` and `Result<T, E>`
/// return types.
pub(super) fn fallible_error(sig: &Signature) -> Option<FallibleError> {
    let ReturnType::Type(_, ty) = &sig.output else {
        return None;
    };
    let Type::Path(type_path) = ty.as_ref() else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    let types = arguments
        .args
        .iter()
        .filter_map(|argument| match argument {
            GenericArgument::Type(ty) => Some(ty),
            _ => None,
        })
        .collect::<Vec<_>>();

    if segment.ident == "ViewResult" && types.len() == 1 {
        return Some(FallibleError::ViewError);
    }
    if segment.ident == "Result" && types.len() == 2 {
        return Some(FallibleError::Explicit(Box::new(types[1].clone())));
    }

    None
}

/// Parses one function parameter as a component prop.
fn parse_prop(input: &FnArg) -> syn::Result<Prop> {
    let FnArg::Typed(PatType { attrs, pat, ty, .. }) = input else {
        return Err(Error::new_spanned(
            input,
            "#[component] functions cannot take self parameters",
        ));
    };

    let Pat::Ident(pat_ident) = pat.as_ref() else {
        return Err(Error::new_spanned(
            pat,
            "#[component] prop parameters must use identifier patterns",
        ));
    };

    if pat_ident.by_ref.is_some() || pat_ident.mutability.is_some() || pat_ident.subpat.is_some() {
        return Err(Error::new_spanned(
            pat,
            "#[component] prop parameters must use plain identifiers",
        ));
    }

    let options = parse_prop_attrs(attrs)?;
    let copied_attrs = attrs
        .iter()
        .filter(|attr| !attr.path().is_ident("prop"))
        .cloned()
        .collect();

    Ok(Prop {
        attrs: copied_attrs,
        ident: pat_ident.ident.clone(),
        ty: ty.clone(),
        default: options.default,
        into: options.into,
    })
}

/// Parsed `#[prop(...)]` options.
#[derive(Default)]
struct PropOptions {
    default: Option<PropDefault>,
    into: bool,
}

/// Parses all `#[prop(...)]` attributes on a parameter.
fn parse_prop_attrs(attrs: &[Attribute]) -> syn::Result<PropOptions> {
    let mut options = PropOptions::default();

    for attr in attrs {
        if !attr.path().is_ident("prop") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("optional") {
                set_default(&mut options, PropDefault::Optional, meta.input)?;
                return Ok(());
            }

            if meta.path.is_ident("default") {
                let value = meta.value()?;
                set_default(
                    &mut options,
                    PropDefault::Expr(Box::new(value.parse()?)),
                    value,
                )?;
                return Ok(());
            }

            if meta.path.is_ident("into") {
                if options.into {
                    return Err(meta.error("duplicate prop option `into`"));
                }

                options.into = true;
                return Ok(());
            }

            Err(meta.error("unsupported prop option; expected optional, default, or into"))
        })?;
    }

    Ok(options)
}

/// Records a prop default, rejecting duplicate defaulting options.
fn set_default(
    options: &mut PropOptions,
    default: PropDefault,
    span: ParseStream<'_>,
) -> syn::Result<()> {
    if options.default.is_some() {
        return Err(span.error("duplicate prop default option"));
    }

    options.default = Some(default);
    Ok(())
}

/// Returns whether a type is the unit type.
fn is_unit_type(ty: &Type) -> bool {
    matches!(ty, Type::Tuple(tuple) if tuple.elems.is_empty())
}

/// Converts a snake_case identifier into PascalCase for generated type names.
fn to_pascal_case(value: &str) -> String {
    let mut output = String::new();
    let mut uppercase_next = true;

    for character in value.trim_start_matches("r#").chars() {
        if character == '_' {
            uppercase_next = true;
            continue;
        }

        if uppercase_next {
            output.extend(character.to_uppercase());
            uppercase_next = false;
        } else {
            output.push(character);
        }
    }

    output
}
