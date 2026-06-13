//! Attribute model for `view!` elements.
//!
//! This module parses element attributes and stores the attribute names and
//! values later validated by element expansion.

use proc_macro2::{TokenStream, TokenTree};
use syn::{
    Expr, Ident, LitStr, Result, Token,
    parse::{Parse, ParseStream},
};

use crate::view::utils::parse::parse_braced_expr;

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
            AttrValue::Expr(Box::new(parse_braced_expr(input)?))
        } else if name == "on_press" {
            AttrValue::Expr(Box::new(parse_unbraced_closure(input)?))
        } else {
            return Err(
                input.error("view! attribute values must be string literals or braced expressions")
            );
        };

        Ok(Self { name, value })
    }
}

/// Parses an unbraced callback closure without consuming the enclosing tag's
/// closing `>`.
fn parse_unbraced_closure(input: ParseStream<'_>) -> Result<Expr> {
    let mut tokens = TokenStream::new();

    while !input.is_empty() && !input.peek(Token![>]) && !next_is_attr_assignment(input) {
        let token: TokenTree = input.parse()?;
        tokens.extend([token]);
    }

    if tokens.is_empty() {
        return Err(input.error("view! on_press attribute must be a callback expression"));
    }

    syn::parse2(tokens).map(Expr::Closure)
}

/// Returns whether the next tokens look like another supported attribute.
fn next_is_attr_assignment(input: ParseStream<'_>) -> bool {
    let fork = input.fork();
    let Ok(name) = fork.parse::<Ident>() else {
        return false;
    };

    matches!(
        name.to_string().as_str(),
        "class" | "id" | "style" | "on_press"
    ) && fork.parse::<Token![=]>().is_ok()
}

/// Attribute value details owned by the `Attr` model.
mod attr_value {
    use proc_macro2::TokenStream;
    use quote::quote;
    use syn::{Expr, LitStr};

    /// Parsed element attribute value.
    pub(in crate::view::model) enum AttrValue {
        /// String literal attribute value.
        Literal(LitStr),
        /// Braced Rust expression attribute value.
        Expr(Box<Expr>),
    }

    impl AttrValue {
        /// Expands the attribute value into Rust tokens.
        ///
        /// # Returns
        ///
        /// A [`TokenStream`] containing the literal or expression value.
        pub(in crate::view::model) fn to_tokens(&self) -> TokenStream {
            match self {
                Self::Literal(value) => quote! { #value },
                Self::Expr(value) => quote! { #value },
            }
        }

        /// Returns whether this value came from a string literal.
        ///
        /// # Returns
        ///
        /// A [`bool`] indicating whether the value is a literal.
        pub(in crate::view::model) const fn is_literal(&self) -> bool {
            matches!(self, Self::Literal(_))
        }
    }
}
