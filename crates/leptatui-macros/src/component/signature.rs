use syn::{Error, ReturnType, Signature, Type};

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
pub(super) fn validate(sig: &Signature) -> syn::Result<()> {
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
