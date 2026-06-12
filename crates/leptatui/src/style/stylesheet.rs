//! Stylesheet rule storage and resolution.
//!
//! This module stores ordered style rules and resolves them against node
//! selector metadata, ancestor metadata, inherited styles, and inline style
//! overrides.

use crate::{StyleDeclarations, ThemeVariables, node::StyleMetadata};

use super::{StyleSelector, TuiStyle, selector::Specificity};

/// Style rule pairing a selector with a style overlay.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StyleRule {
    /// Selector used to decide whether the rule applies to a node.
    selector: StyleSelector,
    /// Style overlay applied when the selector matches.
    style: StyleDeclarations,
}

impl StyleRule {
    /// Creates a style rule from a selector and style overlay.
    ///
    /// # Arguments
    ///
    /// * `selector` — Selector used to match node style metadata.
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
/// Rules are resolved by selector specificity, then by insertion order within
/// each specificity group.
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
    /// * `selector` — Selector used to match node style metadata.
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
    /// * `selector` — Selector used to match node style metadata.
    /// * `style` — Style values to overlay when the selector matches.
    pub fn push_rule(&mut self, selector: StyleSelector, style: impl Into<StyleDeclarations>) {
        self.rules.push(StyleRule::new(selector, style));
    }

    /// Resolves the style for a node.
    ///
    /// Starts with inherited style values, overlays matching type, class, and id
    /// rules using the current node and ancestor chain, then overlays any
    /// inline style stored in the node metadata.
    ///
    /// # Arguments
    ///
    /// * `metadata` — Node selector metadata used for rule matching.
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
        let mut resolved = StyleDeclarations::from(inherited);

        self.apply_matching(&mut resolved, metadata, ancestors, Specificity::Type);
        self.apply_matching(&mut resolved, metadata, ancestors, Specificity::Class);
        self.apply_matching(&mut resolved, metadata, ancestors, Specificity::Id);

        if let Some(inline_style) = metadata.inline_style() {
            resolved.overlay(&StyleDeclarations::from(inline_style));
        }

        resolved.resolve(theme)
    }

    /// Applies matching rules at a single selector specificity.
    ///
    /// # Arguments
    ///
    /// * `resolved` — Style being accumulated for the target node.
    /// * `metadata` — Node selector metadata used for rule matching.
    /// * `ancestors` — Ancestor metadata ordered from outermost to innermost.
    /// * `specificity` — Specificity group to apply.
    fn apply_matching(
        &self,
        resolved: &mut StyleDeclarations,
        metadata: &StyleMetadata,
        ancestors: &[StyleMetadata],
        specificity: Specificity,
    ) {
        for rule in &self.rules {
            if rule.selector.specificity() == specificity
                && rule.selector.matches(metadata, ancestors)
            {
                resolved.overlay(&rule.style);
            }
        }
    }
}
