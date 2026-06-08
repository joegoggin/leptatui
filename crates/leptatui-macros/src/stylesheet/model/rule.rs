//! Rule model for `stylesheet!` syntax.
//!
//! This module parses a selector and a non-empty block of declarations, nested
//! rules, or both, then expands them into stylesheet rule builder calls.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Result, Token, braced,
    parse::{Parse, ParseStream},
};

use crate::stylesheet::model::variable::StylesheetVariables;

use super::{declaration::Declaration, selector::Selector};

/// Parsed stylesheet rule.
pub(super) struct Rule {
    /// Selector that determines which nodes receive the style.
    selector: Selector,
    /// Style declarations applied when the selector matches.
    declarations: Vec<Declaration>,
    /// Nested descendant rules applied below this selector.
    nested_rules: Vec<Rule>,
}

impl Parse for Rule {
    /// Parses a stylesheet rule.
    ///
    /// # Arguments
    ///
    /// * `input` — Macro input stream positioned at a selector.
    ///
    /// # Returns
    ///
    /// A [`Rule`] containing the parsed selector, declarations, and nested
    /// rules.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the rule is missing `=>`, has malformed
    /// declaration separators, or contains no declarations or nested rules.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let selector = input.parse()?;
        input.parse::<Token![=>]>()?;

        let content;
        braced!(content in input);

        let mut declarations = Vec::new();
        let mut nested_rules = Vec::new();

        while !content.is_empty() {
            if content.peek(Token![&]) || starts_nested_rule(&content) {
                nested_rules.push(content.parse()?);
            } else {
                declarations.push(content.parse()?);
            }

            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            } else if !content.is_empty()
                && !content.peek(Token![&])
                && !starts_nested_rule(&content)
            {
                return Err(content.error("stylesheet! declarations must be separated by commas"));
            }
        }

        if declarations.is_empty() && nested_rules.is_empty() {
            return Err(
                content.error("stylesheet! rule requires at least one declaration or nested rule")
            );
        }

        Ok(Self {
            selector,
            declarations,
            nested_rules,
        })
    }
}

/// Returns whether the stream starts with a nested rule.
///
/// # Arguments
///
/// * `input` — Macro input stream positioned at a rule body item.
///
/// # Returns
///
/// A [`bool`] indicating whether a selector followed by `=>` is present.
fn starts_nested_rule(input: ParseStream<'_>) -> bool {
    let fork = input.fork();
    fork.parse::<Selector>().is_ok() && fork.peek(Token![=>])
}

impl Rule {
    /// Appends this rule to an in-progress `Stylesheet` expression.
    ///
    /// # Arguments
    ///
    /// * `stylesheet` — Existing stylesheet expression to wrap with this rule.
    /// * `variables` — Stylesheet variables available to rule declarations.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the updated stylesheet expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the selector cannot be expanded, a declaration
    /// name is unsupported, or a referenced stylesheet variable is unknown.
    pub(super) fn expand(
        &self,
        stylesheet: TokenStream,
        variables: &StylesheetVariables<'_>,
    ) -> Result<TokenStream> {
        self.expand_with_parent_path(stylesheet, variables, &[])
    }

    /// Appends this rule and any nested rules using an accumulated selector path.
    ///
    /// # Arguments
    ///
    /// * `stylesheet` — Existing stylesheet expression to wrap with this rule.
    /// * `variables` — Stylesheet variables available to rule declarations.
    /// * `parent_path` — Selector path from outer rules to this rule's parent.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the updated stylesheet expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if a selector path cannot be expanded, a
    /// declaration name is unsupported, or a referenced stylesheet variable is
    /// unknown.
    fn expand_with_parent_path(
        &self,
        mut stylesheet: TokenStream,
        variables: &StylesheetVariables<'_>,
        parent_path: &[&Selector],
    ) -> Result<TokenStream> {
        let mut path = parent_path.to_vec();
        path.push(&self.selector);

        if !self.declarations.is_empty() {
            let selector = Selector::expand_path(&path)?;
            let leptatui = crate::utils::crate_path::leptatui();
            let mut style = quote! { #leptatui::TuiStyle::new() };

            for declaration in &self.declarations {
                style = declaration.expand(style, variables)?;
            }

            stylesheet = quote! { (#stylesheet).rule(#selector, #style) };
        }

        for nested_rule in &self.nested_rules {
            stylesheet = nested_rule.expand_with_parent_path(stylesheet, variables, &path)?;
        }

        Ok(stylesheet)
    }
}
