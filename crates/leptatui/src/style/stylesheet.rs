//! Stylesheet rule storage and resolution.
//!
//! This module stores ordered style rules and resolves them against view
//! selector metadata, ancestor metadata, inherited styles, semantic defaults,
//! inline style overrides, and CSS-like declaration importance.

use crate::{MediaQuery, StyleDeclarations, ThemeVariables, ViewportSize, view::StyleMetadata};

use super::{StyleSelector, TuiStyle, selector::Specificity};

/// Style rule pairing a selector with a style overlay.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StyleRule {
    /// Selector used to decide whether the rule applies to a view.
    selector: StyleSelector,
    /// Style overlay applied when the selector matches.
    style: StyleDeclarations,
    /// Optional viewport query that must match before this rule applies.
    media_query: Option<MediaQuery>,
}

impl StyleRule {
    /// Creates a style rule from a selector and style overlay.
    ///
    /// # Arguments
    ///
    /// * `selector` — Selector used to match view style metadata.
    /// * `style` — Style values to overlay when the selector matches.
    ///
    /// # Returns
    ///
    /// A [`StyleRule`] containing the selector and style.
    pub fn new(selector: StyleSelector, style: impl Into<StyleDeclarations>) -> Self {
        Self {
            selector,
            style: style.into(),
            media_query: None,
        }
    }

    /// Adds a viewport media query to this style rule.
    ///
    /// The rule applies only when both the selector and query match.
    pub fn with_media_query(mut self, media_query: MediaQuery) -> Self {
        self.media_query = Some(media_query);
        self
    }
}

/// Ordered collection of style rules.
///
/// Rules are resolved by CSS selector specificity, then by insertion order.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Stylesheet {
    /// Rules registered for style resolution.
    rules: Vec<StyleRule>,
}

impl Stylesheet {
    /// Creates an empty stylesheet.
    ///
    /// # Returns
    ///
    /// A [`Stylesheet`] with no rules.
    pub const fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Creates an empty stylesheet.
    ///
    /// # Returns
    ///
    /// A [`Stylesheet`] with no rules.
    pub const fn empty() -> Self {
        Self::new()
    }

    /// Adds a rule and returns the updated stylesheet.
    ///
    /// # Arguments
    ///
    /// * `selector` — Selector used to match view style metadata.
    /// * `style` — Style values to overlay when the selector matches.
    ///
    /// # Returns
    ///
    /// A [`Stylesheet`] containing the appended rule.
    pub fn rule(mut self, selector: StyleSelector, style: impl Into<StyleDeclarations>) -> Self {
        self.push_rule(selector, style);
        self
    }

    /// Adds a media-gated rule and returns the updated stylesheet.
    ///
    /// # Arguments
    ///
    /// * `media_query` — Viewport query required before the rule applies.
    /// * `selector` — Selector used to match view style metadata.
    /// * `style` — Style values to overlay when the query and selector match.
    ///
    /// # Returns
    ///
    /// A [`Stylesheet`] containing the appended media rule.
    pub fn media_rule(
        mut self,
        media_query: MediaQuery,
        selector: StyleSelector,
        style: impl Into<StyleDeclarations>,
    ) -> Self {
        self.push_media_rule(media_query, selector, style);
        self
    }

    /// Appends a rule to the stylesheet.
    ///
    /// # Arguments
    ///
    /// * `selector` — Selector used to match view style metadata.
    /// * `style` — Style values to overlay when the selector matches.
    pub fn push_rule(&mut self, selector: StyleSelector, style: impl Into<StyleDeclarations>) {
        self.rules.push(StyleRule::new(selector, style));
    }

    /// Appends a media-gated rule to the stylesheet.
    ///
    /// # Arguments
    ///
    /// * `media_query` — Viewport query required before the rule applies.
    /// * `selector` — Selector used to match view style metadata.
    /// * `style` — Style values to overlay when the query and selector match.
    pub fn push_media_rule(
        &mut self,
        media_query: MediaQuery,
        selector: StyleSelector,
        style: impl Into<StyleDeclarations>,
    ) {
        self.rules
            .push(StyleRule::new(selector, style).with_media_query(media_query));
    }

    /// Appends all rules from another stylesheet.
    pub(crate) fn extend(&mut self, stylesheet: &Self) {
        self.rules.extend(stylesheet.rules.iter().cloned());
    }

    /// Resolves the style for a view.
    ///
    /// Starts with inherited style values, overlays low-precedence defaults for
    /// the view type, overlays matching normal rules using CSS selector
    /// specificity and source order, overlays any inline style stored in the
    /// view metadata, then overlays matching important rules.
    ///
    /// # Arguments
    ///
    /// * `metadata` — View selector metadata used for rule matching.
    /// * `ancestors` — Ancestor metadata ordered from outermost to innermost.
    /// * `inherited` — Style values inherited from the parent render context.
    /// * `theme` — Runtime theme variables used to resolve theme-aware values.
    ///
    /// # Returns
    ///
    /// A [`TuiStyle`] containing the resolved style.
    pub fn resolve(
        &self,
        metadata: &StyleMetadata,
        ancestors: &[StyleMetadata],
        inherited: TuiStyle,
        theme: &ThemeVariables,
    ) -> TuiStyle {
        Self::resolve_stylesheets(
            ::std::slice::from_ref(self),
            metadata,
            ancestors,
            inherited,
            metadata.inline_style(),
            None,
            theme,
        )
    }

    /// Resolves the style for a view with viewport-aware media rules enabled.
    ///
    /// # Arguments
    ///
    /// * `metadata` — View selector metadata used for rule matching.
    /// * `ancestors` — Ancestor metadata ordered from outermost to innermost.
    /// * `inherited` — Style values inherited from the parent render context.
    /// * `viewport` — Root terminal size used to match media rules.
    /// * `theme` — Runtime theme variables used to resolve theme-aware values.
    ///
    /// # Returns
    ///
    /// A [`TuiStyle`] containing the resolved style.
    pub fn resolve_for_viewport(
        &self,
        metadata: &StyleMetadata,
        ancestors: &[StyleMetadata],
        inherited: TuiStyle,
        viewport: ViewportSize,
        theme: &ThemeVariables,
    ) -> TuiStyle {
        Self::resolve_stylesheets(
            ::std::slice::from_ref(self),
            metadata,
            ancestors,
            inherited,
            metadata.inline_style(),
            Some(viewport),
            theme,
        )
    }

    /// Resolves styles from an ordered stack of active component stylesheets.
    pub(crate) fn resolve_stylesheets(
        stylesheets: &[Self],
        metadata: &StyleMetadata,
        ancestors: &[StyleMetadata],
        inherited: TuiStyle,
        inline_style: Option<TuiStyle>,
        viewport: Option<ViewportSize>,
        theme: &ThemeVariables,
    ) -> TuiStyle {
        let mut resolved = StyleDeclarations::from(inherited);
        resolved.overlay_normal(&StyleDeclarations::from(
            metadata.view_type().default_style(),
        ));
        let rules = Self::matching_rules(stylesheets, metadata, ancestors, viewport);

        for rule in &rules {
            resolved.overlay_normal(rule.style);
        }

        if let Some(inline_style) = inline_style {
            resolved.overlay_normal(&StyleDeclarations::from(inline_style));
        }

        for rule in &rules {
            resolved.overlay_important(rule.style);
        }

        resolved.resolve(theme)
    }

    /// Returns matching rules in CSS cascade order.
    fn matching_rules<'a>(
        stylesheets: &'a [Self],
        metadata: &StyleMetadata,
        ancestors: &[StyleMetadata],
        viewport: Option<ViewportSize>,
    ) -> Vec<MatchingRule<'a>> {
        let mut rules = Vec::new();
        let mut source_order = 0;

        for stylesheet in stylesheets {
            for rule in &stylesheet.rules {
                let media_matches = rule
                    .media_query
                    .as_ref()
                    .is_none_or(|query| viewport.is_some_and(|viewport| query.matches(viewport)));

                if media_matches && rule.selector.matches(metadata, ancestors) {
                    rules.push(MatchingRule {
                        specificity: rule.selector.specificity(),
                        source_order,
                        style: &rule.style,
                    });
                }

                source_order += 1;
            }
        }

        rules.sort_by_key(|rule| (rule.specificity, rule.source_order));
        rules
    }
}

/// Stylesheet rule that matched a view during cascade resolution.
struct MatchingRule<'a> {
    /// Selector specificity used for cascade ordering.
    specificity: Specificity,
    /// Original rule index used to break equal-specificity ties.
    source_order: usize,
    /// Declarations applied when this rule wins a cascade slot.
    style: &'a StyleDeclarations,
}
