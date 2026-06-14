//! Rule model for `stylesheet!` syntax.
//!
//! This module parses a selector and a non-empty block of declarations, mixin
//! includes, nested rules, or a combination of them, then expands them into
//! stylesheet rule builder calls.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Result, Token, braced,
    parse::{Parse, ParseStream},
};

use crate::stylesheet::model::{
    import::StylesheetImports,
    mixin::{MixinInclude, StylesheetMixins},
    variable::StylesheetVariables,
};

use super::{declaration::Declaration, selector::Selector};

/// Parsed stylesheet rule.
pub(super) struct Rule {
    /// Selector that determines which views receive the style.
    selector: Selector,
    /// Style declarations and mixin includes applied when the selector matches.
    style_items: Vec<StyleItem>,
    /// Nested descendant rules applied below this selector.
    nested_rules: Vec<Rule>,
}

/// Ordered style item inside a rule body.
enum StyleItem {
    /// Ordinary style declaration.
    Declaration(Declaration),
    /// Reusable declaration mixin include.
    MixinInclude(MixinInclude),
}

impl StyleItem {
    /// Appends this item to an in-progress `StyleDeclarations` expression.
    ///
    /// # Arguments
    ///
    /// * `style` — Existing `StyleDeclarations` expression to wrap with this item.
    /// * `variables` — Stylesheet variables available to item values.
    /// * `mixins` — Stylesheet mixins available to item includes.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the updated style expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if a declaration name is unsupported or a
    /// referenced stylesheet variable or mixin is unknown.
    fn expand(
        &self,
        style: TokenStream,
        variables: &StylesheetVariables<'_>,
        imports: &StylesheetImports,
        mixins: &StylesheetMixins<'_>,
    ) -> Result<TokenStream> {
        match self {
            Self::Declaration(declaration) => declaration.expand(style, variables, imports),
            Self::MixinInclude(include) => include.expand(style, variables, imports, mixins),
        }
    }
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
    /// A [`Rule`] containing the parsed selector, style items, and nested rules.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the rule is missing `=>`, has malformed
    /// declaration or include separators, or contains no style items or nested
    /// rules.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let selector = input.parse()?;
        input.parse::<Token![=>]>()?;

        let content;
        braced!(content in input);

        let mut style_items = Vec::new();
        let mut nested_rules = Vec::new();

        while !content.is_empty() {
            if content.peek(Token![&]) || starts_nested_rule(&content) {
                nested_rules.push(content.parse()?);
            } else if content.peek(Token![@]) {
                style_items.push(StyleItem::MixinInclude(content.parse()?));
            } else {
                style_items.push(StyleItem::Declaration(content.parse()?));
            }

            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            } else if !content.is_empty()
                && !content.peek(Token![&])
                && !starts_nested_rule(&content)
            {
                return Err(content.error(
                    "stylesheet! declarations and mixin includes must be separated by commas",
                ));
            }
        }

        if style_items.is_empty() && nested_rules.is_empty() {
            return Err(content.error(
                "stylesheet! rule requires at least one declaration, mixin include, or nested rule",
            ));
        }

        Ok(Self {
            selector,
            style_items,
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
    /// * `mixins` — Stylesheet mixins available to rule includes.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the updated stylesheet expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the selector cannot be expanded, a declaration
    /// name is unsupported, or a referenced stylesheet variable or mixin is
    /// unknown.
    pub(super) fn expand(
        &self,
        stylesheet: TokenStream,
        variables: &StylesheetVariables<'_>,
        imports: &StylesheetImports,
        mixins: &StylesheetMixins<'_>,
    ) -> Result<TokenStream> {
        self.expand_with_parent_path(stylesheet, variables, imports, mixins, &[], None)
    }

    /// Appends this rule and descendants as media-gated stylesheet rules.
    pub(super) fn expand_with_media(
        &self,
        stylesheet: TokenStream,
        variables: &StylesheetVariables<'_>,
        imports: &StylesheetImports,
        mixins: &StylesheetMixins<'_>,
        media_query: &TokenStream,
    ) -> Result<TokenStream> {
        self.expand_with_parent_path(
            stylesheet,
            variables,
            imports,
            mixins,
            &[],
            Some(media_query),
        )
    }

    /// Appends this rule and any nested rules using an accumulated selector path.
    ///
    /// # Arguments
    ///
    /// * `stylesheet` — Existing stylesheet expression to wrap with this rule.
    /// * `variables` — Stylesheet variables available to rule declarations.
    /// * `mixins` — Stylesheet mixins available to rule includes.
    /// * `parent_path` — Selector path from outer rules to this rule's parent.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the updated stylesheet expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if a selector path cannot be expanded, a
    /// declaration name is unsupported, or a referenced stylesheet variable or
    /// mixin is unknown.
    fn expand_with_parent_path(
        &self,
        mut stylesheet: TokenStream,
        variables: &StylesheetVariables<'_>,
        imports: &StylesheetImports,
        mixins: &StylesheetMixins<'_>,
        parent_path: &[&Selector],
        media_query: Option<&TokenStream>,
    ) -> Result<TokenStream> {
        let mut path = parent_path.to_vec();
        path.push(&self.selector);

        if !self.style_items.is_empty() {
            let selector = Selector::expand_path(&path)?;
            let leptatui = crate::utils::crate_path::leptatui();
            let mut style = quote! { #leptatui::StyleDeclarations::new() };

            for item in &self.style_items {
                style = item.expand(style, variables, imports, mixins)?;
            }

            stylesheet = if let Some(media_query) = media_query {
                quote! { (#stylesheet).media_rule(#media_query, #selector, #style) }
            } else {
                quote! { (#stylesheet).rule(#selector, #style) }
            };
        }

        for nested_rule in &self.nested_rules {
            stylesheet = nested_rule.expand_with_parent_path(
                stylesheet,
                variables,
                imports,
                mixins,
                &path,
                media_query,
            )?;
        }

        Ok(stylesheet)
    }
}
