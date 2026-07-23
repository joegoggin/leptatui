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

        macro_rules! overlay_layout {
            ($field:ident, $setter:ident) => {
                if let Some(declaration) = &style.$field
                    && matches(declaration.important)
                {
                    self.$setter(declaration.value, declaration.important);
                }
            };
        }

        overlay_layout!(display, set_display);
        overlay_layout!(box_sizing, set_box_sizing);
        overlay_layout!(overflow, set_overflow);
        overlay_layout!(size, set_size);
        overlay_layout!(min_size, set_min_size);
        overlay_layout!(max_size, set_max_size);
        overlay_layout!(margin, set_margin);
        overlay_layout!(gap, set_gap);
        overlay_layout!(flex_direction, set_flex_direction);
        overlay_layout!(flex_wrap, set_flex_wrap);
        overlay_layout!(flex_basis, set_flex_basis);
        overlay_layout!(flex_grow, set_flex_grow);
        overlay_layout!(flex_shrink, set_flex_shrink);
        overlay_layout!(align_items, set_align_items);
        overlay_layout!(align_self, set_align_self);
        overlay_layout!(align_content, set_align_content);
        overlay_layout!(justify_items, set_justify_items);
        overlay_layout!(justify_self, set_justify_self);
        overlay_layout!(justify_content, set_justify_content);
        overlay_layout!(grid_auto_flow, set_grid_auto_flow);
        overlay_layout!(grid_row, set_grid_row);
        overlay_layout!(grid_column, set_grid_column);
        overlay_layout!(position, set_position);
        overlay_layout!(inset, set_inset);
        overlay_layout!(z_index, set_z_index);
    }
}
