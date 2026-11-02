//! Text content model for `view!` elements.
//!
//! This module parses literal and braced-expression content accepted by text
//! and button elements.

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
