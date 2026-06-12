//! Style declaration model for `stylesheet!` syntax.
//!
//! This module parses property declarations inside a stylesheet rule and
//! expands each accepted declaration value into a `StyleDeclarations` builder
//! call.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Error, Ident, Result, Token,
    parse::{Parse, ParseStream},
};

use crate::stylesheet::model::{value::StyleValue, variable::StylesheetVariables};

/// Parsed style declaration such as `fg: Color::White`.
pub(super) struct Declaration {
    /// Declaration property name.
    name: Ident,
    /// Value assigned to the declaration.
    value: StyleValue,
}

impl Parse for Declaration {
    /// Parses a stylesheet declaration name and value.
    ///
    /// # Arguments
    ///
    /// * `input` — Macro input stream positioned at a declaration name.
    ///
    /// # Returns
    ///
    /// A [`Declaration`] containing the parsed property name and value.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the declaration is missing a colon or value.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![:]>()?;
        let value = input.parse()?;

        Ok(Self { name, value })
    }
}

impl Declaration {
    /// Appends this declaration to an in-progress `StyleDeclarations` expression.
    ///
    /// # Arguments
    ///
    /// * `style` — Existing `StyleDeclarations` expression to wrap with this
    ///   declaration.
    /// * `variables` — Stylesheet variables available to declaration values.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the updated style expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the declaration name is unsupported or a
    /// referenced stylesheet variable is unknown.
    pub(super) fn expand(
        &self,
        style: TokenStream,
        variables: &StylesheetVariables<'_>,
    ) -> Result<TokenStream> {
        let value = self.value.expand(variables)?;

        match self.name.to_string().as_str() {
            "fg" | "foreground" => Ok(quote! { (#style).foreground(#value) }),
            "bg" | "background" => Ok(quote! { (#style).background(#value) }),
            "modifier" => Ok(quote! { (#style).modifier(#value) }),
            "borders" => Ok(quote! { (#style).borders(#value) }),
            "border_type" => Ok(quote! { (#style).border_type(#value) }),
            "padding" => Ok(quote! { (#style).padding(#value) }),
            _ => Err(Error::new_spanned(
                &self.name,
                "unsupported stylesheet declaration; expected fg, foreground, bg, background, modifier, borders, border_type, or padding",
            )),
        }
    }
}
