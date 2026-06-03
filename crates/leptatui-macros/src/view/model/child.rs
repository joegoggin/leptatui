//! Child model for `view!` elements.
//!
//! This module distinguishes nested element children from text-like literal or
//! expression children.

use syn::{
    LitStr, Result, Token,
    parse::{Parse, ParseStream},
};

use super::{element::Element, text_content::TextContent};

/// Parsed child node inside an element.
pub(super) enum Child {
    /// Nested element child.
    Element(Element),
    /// Text literal or expression child.
    Text(TextContent),
}

impl Parse for Child {
    /// Parses a child element or text-like child.
    ///
    /// # Arguments
    ///
    /// * `input` — Macro input stream positioned at child content.
    ///
    /// # Returns
    ///
    /// A [`Child`] containing an element, string literal, or braced expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the next tokens cannot form a supported child.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if input.peek(Token![<]) {
            return Ok(Self::Element(input.parse()?));
        }

        if input.peek(LitStr) || input.peek(syn::token::Brace) {
            return Ok(Self::Text(input.parse()?));
        }

        Err(input.error("expected a child element, string literal, or braced expression"))
    }
}
