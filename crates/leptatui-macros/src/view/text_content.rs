//! Text content model for `view!` elements.
//!
//! This module parses literal and braced-expression content accepted by text
//! and button elements.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Expr, LitStr, Result,
    parse::{Parse, ParseStream},
};

use crate::view::syntax::parse_braced_expr;

/// Parsed text-like content inside `Text` or `Button` elements.
pub(super) enum TextContent {
    /// String literal content.
    Literal(LitStr),
    /// Braced Rust expression content.
    Expr(Box<Expr>),
}

impl Parse for TextContent {
    /// Parses text content from a literal or braced expression.
    ///
    /// # Arguments
    ///
    /// * `input` — Macro input stream positioned at text content.
    ///
    /// # Returns
    ///
    /// A [`TextContent`] value containing the parsed literal or expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the next tokens are not text-like content.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if input.peek(LitStr) {
            return Ok(Self::Literal(input.parse()?));
        }

        if input.peek(syn::token::Brace) {
            return Ok(Self::Expr(Box::new(parse_braced_expr(input)?)));
        }

        Err(input.error("expected string literal or braced expression"))
    }
}

impl TextContent {
    /// Expands text content into an expression suitable for text builders.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing a literal, expression, or invoked closure.
    pub(super) fn expand(&self) -> TokenStream {
        match self {
            Self::Literal(value) => quote! { #value },
            Self::Expr(expr) if matches!(expr.as_ref(), Expr::Closure(_)) => quote! { (#expr)() },
            Self::Expr(expr) => quote! { #expr },
        }
    }
}
