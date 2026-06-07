//! Style declaration model for `stylesheet!` syntax.
//!
//! This module parses property declarations inside a stylesheet rule and
//! expands each accepted declaration into a `TuiStyle` builder call.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Error, Expr, Ident, Result, Token,
    parse::{Parse, ParseStream},
};

/// Parsed style declaration such as `fg: Color::White`.
pub(super) struct Declaration {
    /// Declaration property name.
    name: Ident,
    /// Rust expression assigned to the declaration.
    value: Expr,
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
    /// Appends this declaration to an in-progress `TuiStyle` expression.
    ///
    /// # Arguments
    ///
    /// * `style` — Existing `TuiStyle` expression to wrap with this
    ///   declaration.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the updated style expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the declaration name is unsupported.
    pub(super) fn expand(&self, style: TokenStream) -> Result<TokenStream> {
        let value = &self.value;

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
