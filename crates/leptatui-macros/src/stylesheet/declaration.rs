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

use crate::stylesheet::{
    import::StylesheetImports,
    value::{StyleValue, StyleValueKind},
    variable::StylesheetVariables,
};

mod kw {
    syn::custom_keyword!(important);
}

/// Parsed style declaration such as `fg: Color::White;`.
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
    /// Returns [`syn::Error`] if the declaration is missing a colon, value, or
    /// terminating semicolon.
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

        if !input.peek(Token![;]) {
            return Err(input.error("stylesheet! declarations must end with `;`"));
        }
        input.parse::<Token![;]>()?;

        Ok(Self {
            name,
            value,
            important,
        })
    }
}

/// Parses a Rust value or variable reference before its declaration terminator.
///
/// Groups remain intact, preserving commas and semicolons inside Rust values.
/// The importance marker is separated before Rust parsing so qualified colors
/// such as `Color::Yellow !important` are not interpreted as macro invocations.
///
/// # Arguments
///
/// * `input` — Remaining declaration tokens.
///
/// # Returns
///
/// A [`StyleValue`] containing the authored expression or variable reference.
///
/// # Errors
///
/// Returns [`syn::Error`] if the value is empty, malformed, or lacks a semicolon.
fn parse_value(input: ParseStream<'_>) -> Result<StyleValue> {
    let mut tokens = TokenStream::new();
    while !input.is_empty() && !input.peek(Token![;]) {
        let fork = input.fork();
        if fork.parse::<Token![!]>().is_ok() && fork.peek(kw::important) {
            break;
        }
        tokens.extend(::std::iter::once(input.parse::<TokenTree>()?));
    }
    if tokens.is_empty() {
        return Err(input.error("stylesheet! declaration requires a value"));
    }
    if input.is_empty() {
        return Err(input.error("stylesheet! declarations must end with `;`"));
    }
    syn::parse2(tokens).map_err(|error| {
        Error::new(error.span(), format!("{error}; stylesheet! declarations require a Rust value followed by `;`, not comma separators"))
    })
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

/// Returns the value kind and builder methods for a declaration name.
///
/// # Arguments
///
/// * `name` — Parsed declaration property name.
///
/// # Returns
///
/// A [`tuple`](prim@tuple) containing the expected value kind, normal builder
/// method, and important builder method.
///
/// # Errors
///
/// Returns [`syn::Error`] if the declaration name is unsupported.
fn declaration_target(name: &Ident) -> Result<(StyleValueKind, &'static str, &'static str)> {
    let spelling = name.to_string();
    let Some(&(_, category, normal, important)) = super::properties::PROPERTIES
        .iter()
        .find(|p| p.0 == spelling)
    else {
        let names = super::properties::PROPERTIES
            .iter()
            .map(|p| p.0)
            .collect::<Vec<_>>();
        let (last, rest) = names
            .split_last()
            .expect("stylesheet properties are nonempty");
        return Err(Error::new_spanned(
            name,
            format!(
                "unsupported stylesheet declaration; expected {}, or {last}",
                rest.join(", ")
            ),
        ));
    };
    let kind = match category {
        "Color" => StyleValueKind::Color,
        "Modifier" => StyleValueKind::Modifier,
        "Borders" => StyleValueKind::Borders,
        "BorderType" => StyleValueKind::BorderType,
        "Spacing" => StyleValueKind::Spacing,
        "Size" => StyleValueKind::Size,
        "Display" => StyleValueKind::Display,
        "BoxSizing" => StyleValueKind::BoxSizing,
        "Overflow" => StyleValueKind::Overflow,
        "LayoutSize" => StyleValueKind::LayoutSize,
        "Number" => StyleValueKind::Number,
        "LengthAutoEdges" => StyleValueKind::LengthAutoEdges,
        "Gap" => StyleValueKind::Gap,
        "FlexDirection" => StyleValueKind::FlexDirection,
        "FlexWrap" => StyleValueKind::FlexWrap,
        "Dimension" => StyleValueKind::Dimension,
        "AlignItems" => StyleValueKind::AlignItems,
        "AlignSelf" => StyleValueKind::AlignSelf,
        "AlignContent" => StyleValueKind::AlignContent,
        "JustifyItems" => StyleValueKind::JustifyItems,
        "JustifySelf" => StyleValueKind::JustifySelf,
        "JustifyContent" => StyleValueKind::JustifyContent,
        "GridAutoFlow" => StyleValueKind::GridAutoFlow,
        "GridTemplateTracks" => StyleValueKind::GridTemplateTracks,
        "GridAutoTracks" => StyleValueKind::GridAutoTracks,
        "GridLine" => StyleValueKind::GridLine,
        "Position" => StyleValueKind::Position,
        "ZIndex" => StyleValueKind::ZIndex,
        _ => {
            return Err(Error::new_spanned(
                name,
                "unsupported stylesheet value category",
            ));
        }
    };
    Ok((kind, normal, important))
}
