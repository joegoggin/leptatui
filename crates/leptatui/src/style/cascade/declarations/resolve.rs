//! Theme resolution and terminal-style conversion.

use super::StyleDeclarations;
use crate::style::{ThemeVariables, TuiStyle};

impl StyleDeclarations {
    /// Resolves theme-backed declarations into concrete terminal style values.
    ///
    /// # Arguments
    ///
    /// * `theme` — Active theme variables used for color resolution.
    ///
    /// # Returns
    ///
    /// A [`TuiStyle`] containing the resolved declarations.
    pub(crate) fn resolve(&self, theme: &ThemeVariables) -> TuiStyle {
        let mut style = TuiStyle::new();

        if let Some(color) = &self.foreground {
            style = style.foreground(color.value.resolve(theme));
        }

        if let Some(color) = &self.background {
            style = style.background(color.value.resolve(theme));
        }

        if let Some(modifiers) = &self.modifiers {
            style = style.modifier(modifiers.value);
        }

        if let Some(borders) = &self.borders {
            style = style.borders(borders.value);
        }

        if let Some(border_type) = &self.border_type {
            style = style.border_type(border_type.value);
        }

        if let Some(padding) = &self.padding {
            style = style.padding(padding.value);
        }

        if let Some(direction) = &self.direction {
            style = style.direction(direction.value);
        }

        if let Some(size) = &self.image_size {
            style = style.image_size(size.value);
        }

        macro_rules! resolve_layout {
            ($field:ident) => {
                if let Some(declaration) = &self.$field {
                    style = style.$field(declaration.value);
                }
            };
        }

        resolve_layout!(display);
        resolve_layout!(box_sizing);
        resolve_layout!(overflow);
        resolve_layout!(size);
        resolve_layout!(min_size);
        resolve_layout!(max_size);
        resolve_layout!(margin);
        resolve_layout!(gap);
        resolve_layout!(flex_direction);
        resolve_layout!(flex_wrap);
        resolve_layout!(flex_basis);
        resolve_layout!(flex_grow);
        resolve_layout!(flex_shrink);
        resolve_layout!(align_items);
        resolve_layout!(align_self);
        resolve_layout!(align_content);
        resolve_layout!(justify_items);
        resolve_layout!(justify_self);
        resolve_layout!(justify_content);
        resolve_layout!(grid_auto_flow);
        resolve_layout!(grid_row);
        resolve_layout!(grid_column);
        resolve_layout!(position);
        resolve_layout!(inset);
        resolve_layout!(z_index);

        style
    }
}

impl From<TuiStyle> for StyleDeclarations {
    /// Creates style declarations from concrete terminal style values.
    ///
    /// # Arguments
    ///
    /// * `style` — Concrete terminal style to convert.
    ///
    /// # Returns
    ///
    /// A [`StyleDeclarations`] value containing the present style fields.
    fn from(style: TuiStyle) -> Self {
        let mut declarations = Self::new();

        if let Some(color) = style.foreground {
            declarations = declarations.foreground(color);
        }

        if let Some(color) = style.background {
            declarations = declarations.background(color);
        }

        if let Some(modifiers) = style.modifiers {
            declarations = declarations.modifier(modifiers);
        }

        if let Some(borders) = style.borders {
            declarations = declarations.borders(borders);
        }

        if let Some(border_type) = style.border_type {
            declarations = declarations.border_type(border_type);
        }

        if let Some(padding) = style.padding {
            declarations = declarations.padding(padding);
        }

        if let Some(direction) = style.direction {
            declarations = declarations.direction(direction);
        }

        if let Some(size) = style.image_size {
            declarations = declarations.image_size(size);
        }

        macro_rules! convert_layout {
            ($field:ident) => {
                if let Some(value) = style.$field {
                    declarations = declarations.$field(value);
                }
            };
        }

        convert_layout!(display);
        convert_layout!(box_sizing);
        convert_layout!(overflow);
        convert_layout!(size);
        convert_layout!(min_size);
        convert_layout!(max_size);
        convert_layout!(margin);
        convert_layout!(gap);
        convert_layout!(flex_direction);
        convert_layout!(flex_wrap);
        convert_layout!(flex_basis);
        convert_layout!(flex_grow);
        convert_layout!(flex_shrink);
        convert_layout!(align_items);
        convert_layout!(align_self);
        convert_layout!(align_content);
        convert_layout!(justify_items);
        convert_layout!(justify_self);
        convert_layout!(justify_content);
        convert_layout!(grid_auto_flow);
        convert_layout!(grid_row);
        convert_layout!(grid_column);
        convert_layout!(position);
        convert_layout!(inset);
        convert_layout!(z_index);

        declarations
    }
}
