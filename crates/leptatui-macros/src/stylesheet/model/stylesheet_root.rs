//! Root model for a `stylesheet!` invocation.
//!
//! This module owns the top-level variable and mixin definitions plus the rule
//! list, and enforces that a stylesheet macro invocation contains at least one
//! rule, variable, or mixin.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Result, Token,
    parse::{Parse, ParseStream},
};

use crate::stylesheet::model::{
    import::{StylesheetImports, UseImport, starts_use},
    mixin::{Mixin, StylesheetMixins, starts_mixin},
    variable::{StylesheetVariables, Variable},
};

use super::rule::Rule;

/// Root view for a `stylesheet!` invocation.
pub(in crate::stylesheet) struct StylesheetRoot {
    /// Parsed imported style modules in source order.
    imports: Vec<UseImport>,
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
    /// A [`StylesheetRoot`] containing parsed imports, variables, mixins, and
    /// rules.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if an import, variable, or mixin fails to parse,
    /// no usable stylesheet items are present, or rule parsing fails.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut imports = Vec::new();
        while starts_use(input) {
            imports.push(input.parse()?);
        }

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

        if rules.is_empty() && variables.is_empty() && mixins.is_empty() {
            return Err(input.error("stylesheet! requires at least one rule, variable, or mixin"));
        }

        Ok(Self {
            imports,
            variables,
            mixins,
            rules,
        })
    }
}

impl StylesheetRoot {
    /// Expands the stylesheet or module into generated builder calls.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing either a registered `Stylesheet` expression
    /// or a `StyleModule` expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if an import, variable, or mixin is duplicated or
    /// any rule or module item cannot be expanded.
    pub(in crate::stylesheet) fn expand(self) -> Result<TokenStream> {
        let leptatui = crate::utils::crate_path::leptatui();
        let mut imports = StylesheetImports::default();
        let mut variables = StylesheetVariables::default();
        let mut mixins = StylesheetMixins::default();

        for import in &self.imports {
            imports.insert(import)?;
        }

        let import_bindings = self
            .imports
            .iter()
            .map(|import| import.expand_binding(&imports, &leptatui))
            .collect::<Result<Vec<_>>>()?;

        for variable in &self.variables {
            variables.insert(variable)?;
        }

        for mixin in &self.mixins {
            mixins.insert(mixin)?;
        }

        if self.rules.is_empty() {
            let mut module = quote! { #leptatui::StyleModule::new() };

            for variable in &self.variables {
                let name = variable.name().to_string();
                let value = variable.value();

                module = quote! { (#module).variable(#name, #value) };
            }

            for mixin in &self.mixins {
                let name = mixin.name().to_string();
                let mut style = quote! { #leptatui::StyleDeclarations::new() };

                style = mixin.expand(style, &variables, &imports, &mixins)?;
                module = quote! { (#module).mixin(#name, #style) };
            }

            return Ok(quote! {
                {
                    #(#import_bindings)*
                    #module
                }
            });
        }

        let mut stylesheet = quote! { #leptatui::Stylesheet::new() };

        for rule in &self.rules {
            stylesheet = rule.expand(stylesheet, &variables, &imports, &mixins)?;
        }

        Ok(quote! {
            {
                #(#import_bindings)*
                let __leptatui_stylesheet = #stylesheet;
                #leptatui::__private::__register_stylesheet(&__leptatui_stylesheet);
                __leptatui_stylesheet
            }
        })
    }
}
