//! Attribute model for `view!` elements.
//!
//! This module parses element attributes and stores the attribute names and
//! values later validated by element expansion.

use proc_macro2::{TokenStream, TokenTree};
use syn::{
    Expr, Ident, LitStr, Result, Token,
    parse::{Parse, ParseStream},
};

use crate::view::syntax::{next_is_self_closing_tag_end, parse_braced_expr};

use self::attr_value::AttrValue;

/// Parsed element attribute.
pub(super) struct Attr {
    /// Attribute name accepted by validation.
    pub(super) name: Ident,
    /// Attribute value emitted by expansion.
    pub(super) value: AttrValue,
}

impl Parse for Attr {
    /// Parses an element attribute with a literal or expression value.
    ///
    /// # Arguments
    ///
    /// * `input` — Macro input stream positioned at an attribute name.
    ///
    /// # Returns
    ///
    /// An [`Attr`] containing the parsed attribute name and value.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the attribute is missing `=` or its value is
    /// not a string literal, braced expression, or supported unbraced callback.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![=]>()?;

        let value = if input.peek(LitStr) {
            AttrValue::Literal(input.parse()?)
        } else if input.peek(syn::token::Brace) {
            AttrValue::Expr {
                value: Box::new(parse_braced_expr(input)?),
                braced: true,
            }
        } else {
            AttrValue::Expr {
                value: Box::new(parse_unbraced_expr(input)?),
                braced: false,
            }
        };

        Ok(Self { name, value })
    }
}

/// Parses an unbraced expression without consuming the enclosing tag's closing
/// `>` or `/>`.
fn parse_unbraced_expr(input: ParseStream<'_>) -> Result<Expr> {
    let mut tokens = TokenStream::new();

    while !input.is_empty()
        && !input.peek(Token![>])
        && !next_is_self_closing_tag_end(input)
        && !next_is_attr_assignment(input)
    {
        let token: TokenTree = input.parse()?;
        tokens.extend([token]);
    }

    if tokens.is_empty() {
        return Err(input.error("view! attribute value must be an expression"));
    }

    syn::parse2(tokens)
}

/// Returns whether the next tokens look like another attribute assignment.
fn next_is_attr_assignment(input: ParseStream<'_>) -> bool {
    let fork = input.fork();
    if fork.parse::<Ident>().is_err() {
        return false;
    }

    fork.parse::<Token![=]>().is_ok()
}

/// Attribute value details owned by the `Attr` model.
mod attr_value {
    use proc_macro2::TokenStream;
    use quote::quote;
    use syn::{Expr, LitStr};

    /// Parsed element attribute value.
    pub(in crate::view) enum AttrValue {
        /// String literal attribute value.
        Literal(LitStr),
        /// Braced Rust expression attribute value.
        Expr { value: Box<Expr>, braced: bool },
    }

    impl AttrValue {
        /// Expands the attribute value into Rust tokens.
        ///
        /// # Returns
        ///
        /// A [`TokenStream`] containing the literal or expression value.
        pub(in crate::view) fn to_tokens(&self) -> TokenStream {
            match self {
                Self::Literal(value) => quote! { #value },
                Self::Expr { value, .. } => quote! { #value },
            }
        }

        /// Returns whether this value came from a string literal.
        ///
        /// # Returns
        ///
        /// A [`bool`] indicating whether the value is a literal.
        pub(in crate::view) const fn is_literal(&self) -> bool {
            matches!(self, Self::Literal(_))
        }

        /// Returns whether this value came from an unbraced expression.
        ///
        /// # Returns
        ///
        /// A [`bool`] indicating whether the expression was unbraced.
        pub(in crate::view) const fn is_unbraced_expr(&self) -> bool {
            matches!(self, Self::Expr { braced: false, .. })
        }
    }
}
