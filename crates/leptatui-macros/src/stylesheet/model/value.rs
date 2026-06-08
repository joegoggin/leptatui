//! Style value model for `stylesheet!` syntax.
//!
//! This module parses declaration values as either Rust expressions or
//! references to variables declared earlier in the same stylesheet.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Expr, Result, Token,
    parse::{Parse, ParseStream},
};

use super::variable::{StylesheetVariables, VariableRef};

/// Parsed declaration value.
pub(super) enum StyleValue {
    /// Inline Rust expression assigned directly to a declaration.
    Expr(Expr),
    /// Reference to a stylesheet variable.
    Variable(VariableRef),
}

impl Parse for StyleValue {
    /// Parses a declaration value.
    ///
    /// # Arguments
    ///
    /// * `input` — Macro input stream positioned at a declaration value.
    ///
    /// # Returns
    ///
    /// A [`StyleValue`] containing either an expression or variable reference.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the value is neither a valid expression nor a
    /// valid variable reference.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if input.peek(Token![$]) {
            return Ok(Self::Variable(input.parse()?));
        }

        Ok(Self::Expr(input.parse()?))
    }
}

impl StyleValue {
    /// Expands this value into Rust tokens.
    ///
    /// # Arguments
    ///
    /// * `variables` — Stylesheet variables available to this value.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the expression represented by this value.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if this value references an unknown stylesheet
    /// variable.
    pub(super) fn expand(&self, variables: &StylesheetVariables<'_>) -> Result<TokenStream> {
        match self {
            Self::Expr(value) => Ok(quote! { #value }),
            Self::Variable(variable) => variable.expand(variables),
        }
    }
}
