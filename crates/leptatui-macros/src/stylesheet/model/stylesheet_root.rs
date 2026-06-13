//! Root model for a `stylesheet!` invocation.
//!
//! This module owns the top-level variable and mixin definitions plus the rule
//! list, and enforces that a stylesheet macro invocation contains at least one
//! rule.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Result, Token,
    parse::{Parse, ParseStream},
};

use crate::stylesheet::model::{
    mixin::{Mixin, StylesheetMixins, starts_mixin},
    variable::{StylesheetVariables, Variable},
};

use super::rule::Rule;

/// Root view for a `stylesheet!` invocation.
pub(in crate::stylesheet) struct StylesheetRoot {
    /// Parsed variable definitions in source order.
    variables: Vec<Variable>,
    /// Parsed mixin definitions in source order.
    mixins: Vec<Mixin>,
    /// Parsed style rules in source order.
    rules: Vec<Rule>,
}

impl Parse for StylesheetRoot {
    /// Parses the top-level stylesheet variable definitions, mixins, and rule
    /// list.
    ///
    /// # Arguments
    ///
    /// * `input` — Macro input stream for the full `stylesheet!` invocation.
    ///
    /// # Returns
    ///
    /// A [`StylesheetRoot`] containing parsed variables, mixins, and one or more
    /// rules.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if a variable or mixin fails to parse, no rules
    /// are present, or rule parsing fails.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut variables = Vec::new();
        while input.peek(Token![$]) {
            variables.push(input.parse()?);
        }

        let mut mixins = Vec::new();
        while starts_mixin(input) {
            mixins.push(input.parse()?);
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

        Ok(Self {
            variables,
            mixins,
            rules,
        })
    }
}

impl StylesheetRoot {
    /// Expands the stylesheet into generated builder calls.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing a registered `Stylesheet` expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if a variable or mixin is duplicated or any rule
    /// cannot be expanded.
    pub(in crate::stylesheet) fn expand(self) -> Result<TokenStream> {
        let leptatui = crate::utils::crate_path::leptatui();
        let mut variables = StylesheetVariables::default();
        let mut mixins = StylesheetMixins::default();
        let mut stylesheet = quote! { #leptatui::Stylesheet::new() };

        for variable in &self.variables {
            variables.insert(variable)?;
        }

        for mixin in &self.mixins {
            mixins.insert(mixin)?;
        }

        for rule in &self.rules {
            stylesheet = rule.expand(stylesheet, &variables, &mixins)?;
        }

        Ok(quote! {
            {
                let __leptatui_stylesheet = #stylesheet;
                #leptatui::__private::__register_stylesheet(&__leptatui_stylesheet);
                __leptatui_stylesheet
            }
        })
    }
}
