use syn::{
    Error, Expr, Ident, LitStr, Result, Token, braced,
    parse::{Parse, ParseStream},
};

use super::ast::{Attr, Child, Element, TextContent, ViewRoot};

impl Parse for ViewRoot {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let element = input.parse()?;

        if !input.is_empty() {
            return Err(input.error("view! expects a single root element"));
        }

        Ok(Self { element })
    }
}

impl Parse for Element {
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
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![=]>()?;

        if input.peek(LitStr) {
            let _value: LitStr = input.parse()?;
        } else if input.peek(syn::token::Brace) {
            let content;
            braced!(content in input);
            let _value: Expr = content.parse()?;
        } else {
            return Err(
                input.error("view! attribute values must be string literals or braced expressions")
            );
        }

        Ok(Self { name })
    }
}

impl Parse for Child {
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
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if input.peek(LitStr) {
            return Ok(Self::Literal(input.parse()?));
        }

        if input.peek(syn::token::Brace) {
            let content;
            braced!(content in input);
            return Ok(Self::Expr(Box::new(content.parse()?)));
        }

        Err(input.error("expected string literal or braced expression"))
    }
}

fn next_is_closing_tag(input: ParseStream<'_>) -> bool {
    let fork = input.fork();

    fork.parse::<Token![<]>().is_ok() && fork.parse::<Token![/]>().is_ok()
}
