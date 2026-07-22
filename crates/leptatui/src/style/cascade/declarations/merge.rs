//! Importance-aware declaration merging and overlay behavior.

use super::StyleDeclarations;

impl StyleDeclarations {
    /// Overlays another declaration set and returns the updated declarations.
    ///
    /// # Arguments
    ///
    /// * `style` — Declaration set to cascade over this set.
    ///
    /// # Returns
    ///
    /// A [`StyleDeclarations`] value containing the merged declarations.
    pub fn merge(mut self, style: &Self) -> Self {
        self.overlay(style);
        self
    }

    /// Overlays all declarations from another declaration set.
    ///
    /// # Arguments
    ///
    /// * `style` — Declaration set to cascade over this set.
    pub(crate) fn overlay(&mut self, style: &Self) {
        self.overlay_matching_importance(style, |_| true);
    }

    /// Overlays only normal declarations from another declaration set.
    ///
    /// # Arguments
    ///
    /// * `style` — Declaration set to cascade over this set.
    pub(crate) fn overlay_normal(&mut self, style: &Self) {
        self.overlay_matching_importance(style, |important| !important);
    }

    /// Overlays only important declarations from another declaration set.
    ///
    /// # Arguments
    ///
    /// * `style` — Declaration set to cascade over this set.
    pub(crate) fn overlay_important(&mut self, style: &Self) {
        self.overlay_matching_importance(style, |important| important);
    }

    /// Overlays declarations matching a caller-selected importance predicate.
    ///
    /// # Arguments
    ///
    /// * `style` — Declaration set to cascade over this set.
    /// * `matches` — Predicate that accepts declarations by importance.
    fn overlay_matching_importance(&mut self, style: &Self, matches: impl Fn(bool) -> bool) {
        if let Some(declaration) = &style.foreground
            && matches(declaration.important)
        {
            self.set_foreground(declaration.value.clone(), declaration.important);
        }

        if let Some(declaration) = &style.background
            && matches(declaration.important)
        {
            self.set_background(declaration.value.clone(), declaration.important);
        }

        if let Some(declaration) = &style.modifiers
            && matches(declaration.important)
        {
            self.set_modifier(declaration.value, declaration.important);
        }

        if let Some(declaration) = &style.borders
            && matches(declaration.important)
        {
            self.set_borders(declaration.value, declaration.important);
        }

        if let Some(declaration) = &style.border_type
            && matches(declaration.important)
        {
            self.set_border_type(declaration.value, declaration.important);
        }

        if let Some(declaration) = &style.padding
            && matches(declaration.important)
        {
            self.set_padding(declaration.value, declaration.important);
        }

        if let Some(declaration) = &style.direction
            && matches(declaration.important)
        {
            self.set_direction(declaration.value, declaration.important);
        }

        if let Some(declaration) = &style.image_size
            && matches(declaration.important)
        {
            self.set_image_size(declaration.value, declaration.important);
        }
    }
}
