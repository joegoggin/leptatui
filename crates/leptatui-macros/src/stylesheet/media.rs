//! Media query model for `stylesheet!` syntax.
//!
//! This module parses viewport queries and top-level `@media` blocks. Rules
//! reuse the query parser for media blocks nested inside selector bodies.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    Error, Ident, LitInt, Result, Token, braced, parenthesized,
    parse::{Parse, ParseStream},
};

use crate::stylesheet::{
    import::StylesheetImports, mixin::StylesheetMixins, rule::Rule, variable::StylesheetVariables,
};

mod kw {
    syn::custom_keyword!(and);
    syn::custom_keyword!(media);
}

/// Parsed top-level `@media` block.
pub(super) struct MediaBlock {
    /// Viewport query that gates the contained rules.
    query: MediaQuery,
    /// Rules appended when the query matches.
    rules: Vec<Rule>,
}

/// Parsed media query containing one or more `and`-combined conditions.
pub(super) struct MediaQuery {
    conditions: Vec<MediaCondition>,
}

/// Parsed terminal viewport condition.
enum MediaCondition {
    MinWidth(u16),
    MaxWidth(u16),
    MinHeight(u16),
    MaxHeight(u16),
}

impl Parse for MediaBlock {
    /// Parses a top-level `@media (...) { ... }` block.
    ///
    /// # Arguments
    ///
    /// * `input` — Macro input stream positioned at the `@media` token.
    ///
    /// # Returns
    ///
    /// A [`MediaBlock`] containing the viewport query and gated rules.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the query or rules are malformed or the block
    /// contains no rules.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let (query, _) = parse_media_query(input)?;

        let content;
        braced!(content in input);

        let mut rules = Vec::new();
        while !content.is_empty() {
            rules.push(content.parse()?);

            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }

        if rules.is_empty() {
            return Err(content.error("stylesheet! @media block requires at least one rule"));
        }

        Ok(Self { query, rules })
    }
}

impl Parse for MediaQuery {
    /// Parses `(feature: value) and (feature: value)` query syntax.
    ///
    /// # Arguments
    ///
    /// * `input` — Macro input stream positioned at the first condition.
    ///
    /// # Returns
    ///
    /// A [`MediaQuery`] containing one or more viewport conditions.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if any condition is malformed.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut conditions = vec![input.parse()?];

        while input.peek(kw::and) {
            input.parse::<kw::and>()?;
            conditions.push(input.parse()?);
        }

        Ok(Self { conditions })
    }
}

impl Parse for MediaCondition {
    /// Parses one parenthesized terminal viewport condition.
    ///
    /// # Arguments
    ///
    /// * `input` — Macro input stream positioned at the condition.
    ///
    /// # Returns
    ///
    /// A [`MediaCondition`] containing the supported feature and cell count.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the feature is unsupported, the syntax is
    /// malformed, or the value does not fit in a [`u16`].
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let content;
        parenthesized!(content in input);

        let prefix: Ident = content.parse()?;
        content.parse::<Token![-]>()?;
        let dimension: Ident = content.parse()?;
        content.parse::<Token![:]>()?;
        let value: LitInt = content.parse()?;

        if !content.is_empty() {
            return Err(content.error(
                "stylesheet! media query condition must contain exactly one feature comparison",
            ));
        }

        let value = value.base10_parse::<u16>().map_err(|_| {
            Error::new_spanned(&value, "stylesheet! media query value must fit in a u16")
        })?;

        match (prefix.to_string().as_str(), dimension.to_string().as_str()) {
            ("min", "width") => Ok(Self::MinWidth(value)),
            ("max", "width") => Ok(Self::MaxWidth(value)),
            ("min", "height") => Ok(Self::MinHeight(value)),
            ("max", "height") => Ok(Self::MaxHeight(value)),
            _ => Err(Error::new_spanned(
                prefix,
                "unsupported stylesheet media feature; expected min-width, max-width, min-height, or max-height",
            )),
        }
    }
}

/// Returns whether the stream starts with an `@media` block.
///
/// # Arguments
///
/// * `input` — Macro input stream to inspect without consuming.
///
/// # Returns
///
/// A [`bool`] indicating whether `@media` is next.
pub(super) fn starts_media(input: ParseStream<'_>) -> bool {
    let fork = input.fork();
    fork.parse::<Token![@]>().is_ok() && fork.parse::<kw::media>().is_ok()
}

/// Parses an `@media` prefix and its viewport query.
///
/// # Arguments
///
/// * `input` — Macro input stream positioned at the `@media` token.
///
/// # Returns
///
/// A [`MediaQuery`] and source [`Span`] for the media block.
///
/// # Errors
///
/// Returns [`syn::Error`] if the media keyword or query is malformed.
pub(super) fn parse_media_query(input: ParseStream<'_>) -> Result<(MediaQuery, Span)> {
    let span = input.span();
    input.parse::<Token![@]>()?;
    input.parse::<kw::media>()?;
    Ok((input.parse()?, span))
}

impl MediaBlock {
    /// Appends this media block's rules to an in-progress stylesheet expression.
    ///
    /// # Arguments
    ///
    /// * `stylesheet` — Existing stylesheet expression to wrap with this block.
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
    /// Returns [`syn::Error`] if a contained rule cannot be expanded.
    pub(super) fn expand(
        &self,
        mut stylesheet: TokenStream,
        variables: &StylesheetVariables<'_>,
        imports: &StylesheetImports,
        mixins: &StylesheetMixins<'_>,
    ) -> Result<TokenStream> {
        let query = self.query.expand();

        for rule in &self.rules {
            stylesheet = rule.expand_with_media(stylesheet, variables, imports, mixins, &query)?;
        }

        Ok(stylesheet)
    }
}

impl MediaQuery {
    /// Expands this query into a public `MediaQuery` builder expression.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the viewport-query builder expression.
    pub(super) fn expand(&self) -> TokenStream {
        let leptatui = crate::crate_path::leptatui();
        let mut conditions = self.conditions.iter();
        let Some(first) = conditions.next() else {
            return quote! { #leptatui::MediaQuery::new() };
        };

        let mut query = first.expand(&leptatui);
        for condition in conditions {
            let condition = condition.expand(&leptatui);
            query = quote! { (#query).and(#condition) };
        }

        query
    }
}

impl MediaCondition {
    /// Expands this condition into a public `MediaQuery` builder expression.
    ///
    /// # Arguments
    ///
    /// * `leptatui` — Token path to the Leptatui crate used in generated code.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the condition builder expression.
    fn expand(&self, leptatui: &TokenStream) -> TokenStream {
        match self {
            Self::MinWidth(width) => quote! { #leptatui::MediaQuery::min_width(#width) },
            Self::MaxWidth(width) => quote! { #leptatui::MediaQuery::max_width(#width) },
            Self::MinHeight(height) => quote! { #leptatui::MediaQuery::min_height(#height) },
            Self::MaxHeight(height) => quote! { #leptatui::MediaQuery::max_height(#height) },
        }
    }
}
