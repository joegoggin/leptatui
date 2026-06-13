//! Stylesheet rule storage and resolution.
//!
//! This module stores ordered style rules and resolves them against view
//! selector metadata, ancestor metadata, inherited styles, inline style
//! overrides, and CSS-like declaration importance.

use crate::{StyleDeclarations, ThemeVariables, view::StyleMetadata};

use super::{StyleSelector, TuiStyle, selector::Specificity};

/// Style rule pairing a selector with a style overlay.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StyleRule {
    /// Selector used to decide whether the rule applies to a view.
    selector: StyleSelector,
    /// Style overlay applied when the selector matches.
    style: StyleDeclarations,
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
        }
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

    /// Appends a rule to the stylesheet.
    ///
    /// # Arguments
    ///
    /// * `selector` — Selector used to match view style metadata.
    /// * `style` — Style values to overlay when the selector matches.
    pub fn push_rule(&mut self, selector: StyleSelector, style: impl Into<StyleDeclarations>) {
        self.rules.push(StyleRule::new(selector, style));
    }

    /// Appends all rules from another stylesheet.
    pub(crate) fn extend(&mut self, stylesheet: &Self) {
        self.rules.extend(stylesheet.rules.iter().cloned());
    }

    /// Resolves the style for a view.
    ///
    /// Starts with inherited style values, overlays matching normal rules using
    /// CSS selector specificity and source order, overlays any inline style
    /// stored in the view metadata, then overlays matching important rules.
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
        theme: &ThemeVariables,
    ) -> TuiStyle {
        let mut resolved = StyleDeclarations::from(inherited);
        let rules = Self::matching_rules(stylesheets, metadata, ancestors);

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
    ) -> Vec<MatchingRule<'a>> {
        let mut rules = Vec::new();
        let mut source_order = 0;

        for stylesheet in stylesheets {
            for rule in &stylesheet.rules {
                if rule.selector.matches(metadata, ancestors) {
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

struct MatchingRule<'a> {
    specificity: Specificity,
    source_order: usize,
    style: &'a StyleDeclarations,
}
