//! Parsing for `view!` macro input.
//!
//! This module implements `syn` parsers for the small XML-like syntax consumed
//! by `view!` before expansion validates supported elements.

use syn::{
    Error, Expr, Ident, LitStr, Result, Token, braced,
    parse::{Parse, ParseStream},
};

use super::ast::{Attr, Child, Element, TextContent, ViewRoot};

impl Parse for ViewRoot {
    /// Parses a `view!` invocation with exactly one root element.
    ///
    /// # Arguments
    ///
    /// * `input` — Macro input stream to parse.
    ///
    /// # Returns
    ///
    /// A [`ViewRoot`] containing the parsed root element.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if parsing fails or tokens remain after the root
    /// element.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let element = input.parse()?;

        if !input.is_empty() {
            return Err(input.error("view! expects a single root element"));
        }

        Ok(Self { element })
    }
}

impl Parse for Element {
    /// Parses an opening tag, children, and matching closing tag.
    ///
    /// # Arguments
    ///
    /// * `input` — Macro input stream positioned at an opening `<`.
    ///
    /// # Returns
    ///
    /// An [`Element`] containing the tag name, attributes, and children.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the element starts with a closing tag, has
    /// invalid syntax, or closes with a mismatched tag name.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        input.parse::<Token![<]>()?;

        if input.peek(Token![/]) {
            return Err(input.error("view! element cannot start with a closing tag"));
        }

        let name: Ident = input.parse()?;
        let mut attrs = Vec::new();

        while !input.peek(Token![>]) {
            attrs.push(input.parse()?);
        }

        input.parse::<Token![>]>()?;

        let mut children = Vec::new();
        while !input.is_empty() && !next_is_closing_tag(input) {
            children.push(input.parse()?);
        }

        input.parse::<Token![<]>()?;
        input.parse::<Token![/]>()?;
        let closing_name: Ident = input.parse()?;
        input.parse::<Token![>]>()?;

        if closing_name != name {
            return Err(Error::new_spanned(
                closing_name,
                format!("expected closing tag </{}>", name),
            ));
        }

        Ok(Self {
            name,
            attrs,
            children,
        })
    }
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

/// Parses braced content as exactly one Rust expression.
///
/// # Arguments
///
/// * `input` — Macro input stream positioned at a braced expression.
///
/// # Returns
///
/// An [`Expr`] parsed from inside the braces.
///
/// # Errors
///
/// Returns [`syn::Error`] if the braced content is missing, invalid, or contains
/// tokens after the expression.
fn parse_braced_expr(input: ParseStream<'_>) -> Result<Expr> {
    let content;
    braced!(content in input);
    let value = content.parse()?;

    if !content.is_empty() {
        return Err(content.error("view! braced content must be a single Rust expression"));
    }

    Ok(value)
}

/// Returns whether the next tokens begin a closing tag.
///
/// # Arguments
///
/// * `input` — Macro input stream to inspect without consuming.
///
/// # Returns
///
/// A [`bool`] indicating whether the stream begins with `</`.
fn next_is_closing_tag(input: ParseStream<'_>) -> bool {
    let fork = input.fork();

    fork.parse::<Token![<]>().is_ok() && fork.parse::<Token![/]>().is_ok()
}
