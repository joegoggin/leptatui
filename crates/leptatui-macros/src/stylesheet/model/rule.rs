//! Rule model for `stylesheet!` syntax.
//!
//! This module parses a selector and non-empty declaration block, then expands
//! the pair into a stylesheet rule builder call.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Result, Token, braced,
    parse::{Parse, ParseStream},
};

use super::{declaration::Declaration, selector::Selector};

/// Parsed stylesheet rule.
pub(super) struct Rule {
    /// Selector that determines which nodes receive the style.
    selector: Selector,
    /// Style declarations applied when the selector matches.
    declarations: Vec<Declaration>,
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
    /// A [`Rule`] containing the parsed selector and declarations.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the rule is missing `=>`, has malformed
    /// declaration separators, or contains no declarations.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let selector = input.parse()?;
        input.parse::<Token![=>]>()?;

        let content;
        braced!(content in input);

        let mut declarations = Vec::new();
        while !content.is_empty() {
            declarations.push(content.parse()?);

            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            } else if !content.is_empty() {
                return Err(content.error("stylesheet! declarations must be separated by commas"));
            }
        }

        if declarations.is_empty() {
            return Err(content.error("stylesheet! rule requires at least one declaration"));
        }

        Ok(Self {
            selector,
            declarations,
        })
    }
}

impl Rule {
    /// Appends this rule to an in-progress `Stylesheet` expression.
    ///
    /// # Arguments
    ///
    /// * `stylesheet` — Existing stylesheet expression to wrap with this rule.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the updated stylesheet expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the selector or any declaration cannot be
    /// expanded.
    pub(super) fn expand(&self, stylesheet: TokenStream) -> Result<TokenStream> {
        let selector = self.selector.expand()?;
        let leptatui = crate::utils::crate_path::leptatui();
        let mut style = quote! { #leptatui::TuiStyle::new() };

        for declaration in &self.declarations {
            style = declaration.expand(style)?;
        }

        Ok(quote! { (#stylesheet).rule(#selector, #style) })
    }
}
