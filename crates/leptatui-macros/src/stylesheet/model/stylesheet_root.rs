//! Root model for a `stylesheet!` invocation.
//!
//! This module owns the top-level variable definitions and rule list, and
//! enforces that a stylesheet macro invocation contains at least one rule.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Result, Token,
    parse::{Parse, ParseStream},
};

use crate::stylesheet::model::variable::{StylesheetVariables, Variable};

use super::rule::Rule;

/// Root node for a `stylesheet!` invocation.
pub(in crate::stylesheet) struct StylesheetRoot {
    /// Parsed variable definitions in source order.
    variables: Vec<Variable>,
    /// Parsed style rules in source order.
    rules: Vec<Rule>,
}

impl Parse for StylesheetRoot {
    /// Parses the top-level stylesheet variable definitions and rule list.
    ///
    /// # Arguments
    ///
    /// * `input` — Macro input stream for the full `stylesheet!` invocation.
    ///
    /// # Returns
    ///
    /// A [`StylesheetRoot`] containing parsed variables and one or more rules.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if a variable fails to parse, no rules are
    /// present, or rule parsing fails.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut variables = Vec::new();
        while input.peek(Token![$]) {
            variables.push(input.parse()?);
        }

        let mut rules = Vec::new();
        while !input.is_empty() {
            rules.push(input.parse()?);

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        if rules.is_empty() {
            return Err(input.error("stylesheet! requires at least one rule"));
        }

        Ok(Self { variables, rules })
    }
}

impl StylesheetRoot {
    /// Expands the stylesheet into generated builder calls.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing a `Stylesheet::new()` builder chain.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if a variable is duplicated or any rule cannot be
    /// expanded.
    pub(in crate::stylesheet) fn expand(self) -> Result<TokenStream> {
        let leptatui = crate::utils::crate_path::leptatui();
        let mut variables = StylesheetVariables::default();
        let mut stylesheet = quote! { #leptatui::Stylesheet::new() };

        for variable in &self.variables {
            variables.insert(variable)?;
        }

        for rule in &self.rules {
            stylesheet = rule.expand(stylesheet, &variables)?;
        }

        Ok(stylesheet)
    }
}
