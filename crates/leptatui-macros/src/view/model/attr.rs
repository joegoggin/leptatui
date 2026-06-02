use syn::{
    Ident, LitStr, Result, Token,
    parse::{Parse, ParseStream},
};

use super::parse_braced_expr;

/// Parsed element attribute.
pub(super) struct Attr {
    /// Attribute name accepted by validation.
    pub(super) name: Ident,
}

impl Parse for Attr {
    /// Parses an element attribute with a literal or expression value.
    ///
    /// # Arguments
    ///
    /// * `input` - Macro input stream positioned at an attribute name.
    ///
    /// # Returns
    ///
    /// An [`Attr`] containing the parsed attribute name.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the attribute is missing `=` or its value is
    /// not a string literal or braced expression.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![=]>()?;

        if input.peek(LitStr) {
            let _value: LitStr = input.parse()?;
        } else if input.peek(syn::token::Brace) {
            let _value = parse_braced_expr(input)?;
        } else {
            return Err(
                input.error("view! attribute values must be string literals or braced expressions")
            );
        }

        Ok(Self { name })
    }
}
