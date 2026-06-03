use crate::node::StyleMetadata;

use super::{StyleSelector, TuiStyle, selector::Specificity};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StyleRule {
    selector: StyleSelector,
    style: TuiStyle,
}

impl StyleRule {
    pub fn new(selector: StyleSelector, style: TuiStyle) -> Self {
        Self { selector, style }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Stylesheet {
    rules: Vec<StyleRule>,
}

impl Stylesheet {
    pub const fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub const fn empty() -> Self {
        Self::new()
    }

    pub fn rule(mut self, selector: StyleSelector, style: TuiStyle) -> Self {
        self.push_rule(selector, style);
        self
    }

    pub fn push_rule(&mut self, selector: StyleSelector, style: TuiStyle) {
        self.rules.push(StyleRule::new(selector, style));
    }

    pub fn resolve(&self, metadata: &StyleMetadata, inherited: TuiStyle) -> TuiStyle {
        let mut resolved = inherited;

        self.apply_matching(&mut resolved, metadata, Specificity::Type);
        self.apply_matching(&mut resolved, metadata, Specificity::Class);
        self.apply_matching(&mut resolved, metadata, Specificity::Id);

        if let Some(inline_style) = metadata.inline_style() {
            resolved.overlay(inline_style);
        }

        resolved
    }

    fn apply_matching(
        &self,
        resolved: &mut TuiStyle,
        metadata: &StyleMetadata,
        specificity: Specificity,
    ) {
        for rule in &self.rules {
            if rule.selector.specificity() == specificity && rule.selector.matches(metadata) {
                resolved.overlay(rule.style);
            }
        }
    }
}
