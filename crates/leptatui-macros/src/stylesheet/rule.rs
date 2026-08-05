//! Rule model for `stylesheet!` syntax.
//!
//! This module parses a selector and a non-empty block of declarations, mixin
//! includes, nested rules, media blocks, or a combination of them, then expands
//! them into stylesheet rule builder calls.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Error, Result, Token, braced,
    parse::{Parse, ParseStream},
};

use crate::stylesheet::{
    import::StylesheetImports,
    media::{MediaQuery, parse_media_query, starts_media},
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
    /// Ordered nested rules and media blocks below this selector.
    nested_items: Vec<NestedItem>,
}

/// Ordered style item inside a rule body.
enum StyleItem {
    /// Ordinary style declaration.
    Declaration(Box<Declaration>),
    /// Reusable declaration mixin include.
    MixinInclude(MixinInclude),
}

/// Ordered structural item inside a rule body.
enum NestedItem {
    /// Descendant or parent-reference rule.
    Rule(Rule),
    /// Viewport-gated declarations and rules.
    Media(NestedMediaBlock),
}

/// Media block nested inside a selector rule.
struct NestedMediaBlock {
    /// Viewport query that gates the contained declarations and rules.
    query: MediaQuery,
    /// Source span used for nested-media diagnostics.
    span: proc_macro2::Span,
    /// Declarations and mixin includes applied to the current selector.
    style_items: Vec<StyleItem>,
    /// Nested selectors scoped beneath the current selector.
    nested_items: Vec<NestedItem>,
}

/// Parsed contents shared by ordinary rules and nested media blocks.
struct RuleBody {
    /// Declarations and mixin includes for the current selector.
    style_items: Vec<StyleItem>,
    /// Ordered nested selectors and media blocks.
    nested_items: Vec<NestedItem>,
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
    /// A [`Rule`] containing the parsed selector, style items, nested rules, and
    /// media blocks.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the rule is missing `=>`, has malformed
    /// declaration or include separators, or contains no style items, nested
    /// rules, or media blocks.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let selector = input.parse()?;
        input.parse::<Token![=>]>()?;

        let content;
        braced!(content in input);

        let RuleBody {
            style_items,
            nested_items,
        } = parse_rule_body(&content, true)?;

        if style_items.is_empty() && nested_items.is_empty() {
            return Err(content.error(
                "stylesheet! rule requires at least one declaration, mixin include, nested rule, or media block",
            ));
        }

        Ok(Self {
            selector,
            style_items,
            nested_items,
        })
    }
}

impl Parse for NestedMediaBlock {
    /// Parses an `@media` block nested within a selector rule.
    ///
    /// # Arguments
    ///
    /// * `input` — Macro input stream positioned at the `@media` token.
    ///
    /// # Returns
    ///
    /// A [`NestedMediaBlock`] containing its query, declarations, and rules.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the query or body is malformed, the block is
    /// empty, or another media block is nested directly within it.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let (query, span) = parse_media_query(input)?;

        let content;
        braced!(content in input);

        let RuleBody {
            style_items,
            nested_items,
        } = parse_rule_body(&content, false)?;

        if style_items.is_empty() && nested_items.is_empty() {
            return Err(content.error(
                "stylesheet! nested @media block requires at least one declaration, mixin include, or nested rule",
            ));
        }

        Ok(Self {
            query,
            span,
            style_items,
            nested_items,
        })
    }
}

/// Parses declarations and structural children from a selector-like body.
///
/// # Arguments
///
/// * `content` — Braced rule-body input stream.
/// * `allow_media` — Whether the body may contain a nested media block.
///
/// # Returns
///
/// A [`RuleBody`] containing declarations and ordered structural children.
///
/// # Errors
///
/// Returns [`syn::Error`] if an item is malformed, separators are missing, or
/// one media block is nested directly inside another.
fn parse_rule_body(content: ParseStream<'_>, allow_media: bool) -> Result<RuleBody> {
    let mut style_items = Vec::new();
    let mut nested_items = Vec::new();

    while !content.is_empty() {
        if starts_media(content) {
            if !allow_media {
                return Err(content.error(
                    "stylesheet! nested @media blocks cannot contain another @media block; combine conditions with `and`",
                ));
            }
            nested_items.push(NestedItem::Media(content.parse()?));
        } else if content.peek(Token![&]) || starts_nested_rule(content) {
            nested_items.push(NestedItem::Rule(content.parse()?));
        } else if content.peek(Token![@]) {
            style_items.push(StyleItem::MixinInclude(content.parse()?));
        } else {
            style_items.push(StyleItem::Declaration(Box::new(content.parse()?)));
        }

        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        } else if !content.is_empty() && !starts_nested_item(content) {
            return Err(content
                .error("stylesheet! declarations and mixin includes must be separated by commas"));
        }
    }

    Ok(RuleBody {
        style_items,
        nested_items,
    })
}

/// Returns whether the stream starts with a nested rule or media block.
///
/// # Arguments
///
/// * `input` — Macro input stream positioned at a rule-body item.
///
/// # Returns
///
/// A [`bool`] indicating whether the next item is structural.
fn starts_nested_item(input: ParseStream<'_>) -> bool {
    starts_media(input) || input.peek(Token![&]) || starts_nested_rule(input)
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
    /// * `imports` — Imported stylesheet modules available to values and mixins.
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
    ///
    /// # Arguments
    ///
    /// * `stylesheet` — Existing stylesheet expression to wrap with this rule.
    /// * `variables` — Stylesheet variables available to rule declarations.
    /// * `imports` — Imported stylesheet modules available to values and mixins.
    /// * `mixins` — Stylesheet mixins available to rule includes.
    /// * `media_query` — Expanded viewport query applied to every emitted rule.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the updated stylesheet expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if a selector, declaration, variable, or mixin
    /// cannot be expanded, or if another media block is nested within the rule.
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
    /// * `imports` — Imported stylesheet modules available to values and mixins.
    /// * `mixins` — Stylesheet mixins available to rule includes.
    /// * `parent_path` — Selector path from outer rules to this rule's parent.
    /// * `media_query` — Optional viewport query inherited from a media block.
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
            let leptatui = crate::crate_path::leptatui();
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

        for nested_item in &self.nested_items {
            stylesheet = match nested_item {
                NestedItem::Rule(nested_rule) => nested_rule.expand_with_parent_path(
                    stylesheet,
                    variables,
                    imports,
                    mixins,
                    &path,
                    media_query,
                )?,
                NestedItem::Media(media) => {
                    if media_query.is_some() {
                        return Err(Error::new(
                            media.span,
                            "stylesheet! nested @media blocks cannot contain another @media block; combine conditions with `and`",
                        ));
                    }
                    media.expand_with_parent_path(stylesheet, variables, imports, mixins, &path)?
                }
            };
        }

        Ok(stylesheet)
    }
}

impl NestedMediaBlock {
    /// Appends this nested media block using the current selector path.
    ///
    /// # Arguments
    ///
    /// * `stylesheet` — Existing stylesheet expression to wrap with this block.
    /// * `variables` — Stylesheet variables available to declarations.
    /// * `imports` — Imported stylesheet modules available to values and mixins.
    /// * `mixins` — Stylesheet mixins available to includes.
    /// * `parent_path` — Selector path inherited from the containing rule.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the updated stylesheet expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if selectors, declarations, variables, or mixins
    /// cannot be expanded.
    fn expand_with_parent_path(
        &self,
        mut stylesheet: TokenStream,
        variables: &StylesheetVariables<'_>,
        imports: &StylesheetImports,
        mixins: &StylesheetMixins<'_>,
        parent_path: &[&Selector],
    ) -> Result<TokenStream> {
        let query = self.query.expand();

        if !self.style_items.is_empty() {
            let selector = Selector::expand_path(parent_path)?;
            let leptatui = crate::crate_path::leptatui();
            let mut style = quote! { #leptatui::StyleDeclarations::new() };

            for item in &self.style_items {
                style = item.expand(style, variables, imports, mixins)?;
            }

            stylesheet = quote! { (#stylesheet).media_rule(#query, #selector, #style) };
        }

        for nested_item in &self.nested_items {
            stylesheet = match nested_item {
                NestedItem::Rule(rule) => rule.expand_with_parent_path(
                    stylesheet,
                    variables,
                    imports,
                    mixins,
                    parent_path,
                    Some(&query),
                )?,
                NestedItem::Media(media) => {
                    return Err(Error::new(
                        media.span,
                        "stylesheet! nested @media blocks cannot contain another @media block; combine conditions with `and`",
                    ));
                }
            };
        }

        Ok(stylesheet)
    }
}
