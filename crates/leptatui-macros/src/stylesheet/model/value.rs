//! Style value model for `stylesheet!` syntax.
//!
//! This module parses declaration values as Rust expressions, local variables,
//! or variables from imported style modules.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Expr, Result, Token,
    parse::{Parse, ParseStream},
};

use super::{
    import::StylesheetImports,
    variable::{ImportedVariableRef, StylesheetVariables, VariableRef, starts_imported_variable},
};

/// Parsed declaration value.
pub(super) enum StyleValue {
    /// Inline Rust expression assigned directly to a declaration.
    Expr(Expr),
    /// Reference to a stylesheet variable.
    Variable(VariableRef),
    /// Reference to a variable from an imported stylesheet module.
    ImportedVariable(ImportedVariableRef),
}

/// Expected declaration value kind for imported module variables.
#[derive(Clone, Copy)]
pub(super) enum StyleValueKind {
    /// A foreground or background color.
    Color,
    /// Text modifier flags.
    Modifier,
    /// Widget border sides.
    Borders,
    /// Widget border glyph set.
    BorderType,
    /// Internal widget padding.
    Spacing,
    /// Child layout direction.
    LayoutDirection,
}

impl Parse for StyleValue {
    /// Parses a declaration value.
    ///
    /// # Arguments
    ///
    /// * `input` — Macro input stream positioned at a declaration value.
    ///
    /// # Returns
    ///
    /// A [`StyleValue`] containing either an expression or variable reference.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the value is neither a valid expression nor a
    /// valid variable reference.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if starts_imported_variable(input) {
            return Ok(Self::ImportedVariable(input.parse()?));
        }

        if input.peek(Token![$]) {
            return Ok(Self::Variable(input.parse()?));
        }

        Ok(Self::Expr(input.parse()?))
    }
}

impl StyleValue {
    /// Expands this value into Rust tokens.
    ///
    /// # Arguments
    ///
    /// * `variables` — Stylesheet variables available to this value.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the expression represented by this value.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if this value references an unknown stylesheet
    /// variable.
    pub(super) fn expand(
        &self,
        variables: &StylesheetVariables<'_>,
        imports: &StylesheetImports,
        kind: StyleValueKind,
    ) -> Result<TokenStream> {
        match self {
            Self::Expr(value) => Ok(quote! { #value }),
            Self::Variable(variable) => variable.expand(variables),
            Self::ImportedVariable(variable) => expand_imported_variable(variable, imports, kind),
        }
    }
}

/// Expands an imported variable reference using the getter for its declaration kind.
fn expand_imported_variable(
    variable: &ImportedVariableRef,
    imports: &StylesheetImports,
    kind: StyleValueKind,
) -> Result<TokenStream> {
    let module = imports.get(variable.alias())?;
    let name = variable.name().to_string();

    Ok(match kind {
        StyleValueKind::Color => quote! { #module.expect_color(#name) },
        StyleValueKind::Modifier => quote! { #module.expect_modifier(#name) },
        StyleValueKind::Borders => quote! { #module.expect_borders(#name) },
        StyleValueKind::BorderType => quote! { #module.expect_border_type(#name) },
        StyleValueKind::Spacing => quote! { #module.expect_spacing(#name) },
        StyleValueKind::LayoutDirection => quote! { #module.expect_layout_direction(#name) },
    })
}
