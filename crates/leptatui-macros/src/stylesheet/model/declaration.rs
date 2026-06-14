//! Style declaration model for `stylesheet!` syntax.
//!
//! This module parses property declarations inside a stylesheet rule and
//! expands each accepted declaration value into a `StyleDeclarations` builder
//! call.

use proc_macro2::{TokenStream, TokenTree};
use quote::quote;
use syn::{
    Error, Ident, Result, Token,
    parse::{Parse, ParseStream},
};

use crate::stylesheet::model::{
    import::StylesheetImports,
    selector::Selector,
    value::{StyleValue, StyleValueKind},
    variable::StylesheetVariables,
};

mod kw {
    syn::custom_keyword!(important);
}

/// Parsed style declaration such as `fg: Color::White`.
pub(super) struct Declaration {
    /// Declaration property name.
    name: Ident,
    /// Value assigned to the declaration.
    value: StyleValue,
    /// Whether this declaration is marked with `!important`.
    important: bool,
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
        let value = parse_value(input)?;
        let important = if input.peek(Token![!]) {
            input.parse::<Token![!]>()?;
            input.parse::<kw::important>()?;
            true
        } else {
            false
        };

        Ok(Self {
            name,
            value,
            important,
        })
    }
}

fn parse_value(input: ParseStream<'_>) -> Result<StyleValue> {
    let mut tokens = TokenStream::new();

    while !input.is_empty()
        && !input.peek(Token![,])
        && !starts_important(input)
        && !starts_nested_rule(input)
    {
        tokens.extend(::std::iter::once(input.parse::<TokenTree>()?));
    }

    if tokens.is_empty() {
        return Err(input.error("stylesheet! declaration requires a value"));
    }

    syn::parse2(tokens)
}

fn starts_important(input: ParseStream<'_>) -> bool {
    let fork = input.fork();
    fork.parse::<Token![!]>().is_ok() && fork.parse::<kw::important>().is_ok()
}

fn starts_nested_rule(input: ParseStream<'_>) -> bool {
    let fork = input.fork();
    fork.parse::<Selector>().is_ok() && fork.peek(Token![=>])
}

impl Declaration {
    /// Appends this declaration to an in-progress `StyleDeclarations` expression.
    ///
    /// # Arguments
    ///
    /// * `style` — Existing `StyleDeclarations` expression to wrap with this
    ///   declaration.
    /// * `variables` — Stylesheet variables available to declaration values.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the updated style expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the declaration name is unsupported or a
    /// referenced stylesheet variable is unknown.
    pub(super) fn expand(
        &self,
        style: TokenStream,
        variables: &StylesheetVariables<'_>,
        imports: &StylesheetImports,
    ) -> Result<TokenStream> {
        let (kind, normal_method, important_method) = declaration_target(&self.name)?;
        let value = self.value.expand(variables, imports, kind)?;
        let method = if self.important {
            important_method
        } else {
            normal_method
        };
        let method = Ident::new(method, self.name.span());

        Ok(quote! { (#style).#method(#value) })
    }
}

fn declaration_target(name: &Ident) -> Result<(StyleValueKind, &'static str, &'static str)> {
    match name.to_string().as_str() {
        "fg" | "foreground" => Ok((StyleValueKind::Color, "foreground", "foreground_important")),
        "bg" | "background" => Ok((StyleValueKind::Color, "background", "background_important")),
        "modifier" => Ok((StyleValueKind::Modifier, "modifier", "modifier_important")),
        "borders" => Ok((StyleValueKind::Borders, "borders", "borders_important")),
        "border_type" => Ok((
            StyleValueKind::BorderType,
            "border_type",
            "border_type_important",
        )),
        "padding" => Ok((StyleValueKind::Spacing, "padding", "padding_important")),
        "direction" => Ok((
            StyleValueKind::LayoutDirection,
            "direction",
            "direction_important",
        )),
        _ => Err(Error::new_spanned(
            name,
            "unsupported stylesheet declaration; expected fg, foreground, bg, background, modifier, borders, border_type, padding, or direction",
        )),
    }
}
