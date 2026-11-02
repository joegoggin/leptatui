//! Style declaration model for `stylesheet!` syntax.
//!
//! This module parses property declarations inside a stylesheet rule and
//! expands each accepted declaration value into a `StyleDeclarations` builder
//! call.

use proc_macro2::{TokenStream, TokenTree};
use quote::quote;
use syn::{
    Error, Ident, Result, Token,
    parse::{Parse, ParseStream},
};

use crate::stylesheet::{
    import::StylesheetImports,
    media::starts_media,
    selector::Selector,
    value::{StyleValue, StyleValueKind},
    variable::StylesheetVariables,
};

mod kw {
    syn::custom_keyword!(important);
}

/// Parsed style declaration such as `fg: Color::White`.
pub(super) struct Declaration {
    /// Declaration property name.
    name: Ident,
    /// Value assigned to the declaration.
    value: StyleValue,
    /// Whether this declaration is marked with `!important`.
    important: bool,
}

impl Parse for Declaration {
    /// Parses a stylesheet declaration name and value.
    ///
    /// # Arguments
    ///
    /// * `input` — Macro input stream positioned at a declaration name.
    ///
    /// # Returns
    ///
    /// A [`Declaration`] containing the parsed property name and value.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the declaration is missing a colon or value.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![:]>()?;
        let value = parse_value(input)?;
        let important = if input.peek(Token![!]) {
            input.parse::<Token![!]>()?;
            input.parse::<kw::important>()?;
            true
        } else {
            false
        };

        Ok(Self {
            name,
            value,
            important,
        })
    }
}

/// Parses declaration value tokens up to a declaration, rule, or media boundary.
///
/// # Arguments
///
/// * `input` — Macro input stream positioned at the first value token.
///
/// # Returns
///
/// A [`StyleValue`] parsed from the collected value tokens.
///
/// # Errors
///
/// Returns [`syn::Error`] if the declaration has no value or the collected
/// tokens do not parse as a supported style value.
fn parse_value(input: ParseStream<'_>) -> Result<StyleValue> {
    let mut tokens = TokenStream::new();

    while !input.is_empty()
        && !input.peek(Token![,])
        && !starts_important(input)
        && !starts_media(input)
        && !starts_nested_rule(input)
    {
        tokens.extend(::std::iter::once(input.parse::<TokenTree>()?));
    }

    if tokens.is_empty() {
        return Err(input.error("stylesheet! declaration requires a value"));
    }

    syn::parse2(tokens)
}

/// Returns whether the input starts with an `!important` marker.
///
/// # Arguments
///
/// * `input` — Macro input stream to inspect without consuming.
///
/// # Returns
///
/// A [`bool`] indicating whether `!important` is next in the stream.
fn starts_important(input: ParseStream<'_>) -> bool {
    let fork = input.fork();
    fork.parse::<Token![!]>().is_ok() && fork.parse::<kw::important>().is_ok()
}

/// Returns whether the input starts with a nested stylesheet rule.
///
/// # Arguments
///
/// * `input` — Macro input stream to inspect without consuming.
///
/// # Returns
///
/// A [`bool`] indicating whether a selector followed by `=>` is next.
fn starts_nested_rule(input: ParseStream<'_>) -> bool {
    let fork = input.fork();
    fork.parse::<Selector>().is_ok() && fork.peek(Token![=>])
}

impl Declaration {
    /// Appends this declaration to an in-progress `StyleDeclarations` expression.
    ///
    /// # Arguments
    ///
    /// * `style` — Existing `StyleDeclarations` expression to wrap with this
    ///   declaration.
    /// * `variables` — Stylesheet variables available to declaration values.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the updated style expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the declaration name is unsupported or a
    /// referenced stylesheet variable is unknown.
    pub(super) fn expand(
        &self,
        style: TokenStream,
        variables: &StylesheetVariables<'_>,
        imports: &StylesheetImports,
    ) -> Result<TokenStream> {
        let (kind, normal_method, important_method) = declaration_target(&self.name)?;
        let value = self.value.expand(variables, imports, kind)?;
        let method = if self.important {
            important_method
        } else {
            normal_method
        };
        let method = Ident::new(method, self.name.span());

        Ok(quote! { (#style).#method(#value) })
    }
}

/// Returns the value kind and builder methods for a declaration name.
///
/// # Arguments
///
/// * `name` — Parsed declaration property name.
///
/// # Returns
///
/// A [`tuple`](prim@tuple) containing the expected value kind, normal builder
/// method, and important builder method.
///
/// # Errors
///
/// Returns [`syn::Error`] if the declaration name is unsupported.
fn declaration_target(name: &Ident) -> Result<(StyleValueKind, &'static str, &'static str)> {
    match name.to_string().as_str() {
        "fg" | "foreground" => Ok((StyleValueKind::Color, "foreground", "foreground_important")),
        "bg" | "background" => Ok((StyleValueKind::Color, "background", "background_important")),
        "modifier" => Ok((StyleValueKind::Modifier, "modifier", "modifier_important")),
        "borders" => Ok((StyleValueKind::Borders, "borders", "borders_important")),
        "border_type" => Ok((
            StyleValueKind::BorderType,
            "border_type",
            "border_type_important",
        )),
        "padding" => Ok((StyleValueKind::Spacing, "padding", "padding_important")),
        "image_size" => Ok((StyleValueKind::Size, "image_size", "image_size_important")),
        "display" => Ok((StyleValueKind::Display, "display", "display_important")),
        "box_sizing" => Ok((
            StyleValueKind::BoxSizing,
            "box_sizing",
            "box_sizing_important",
        )),
        "overflow" => Ok((StyleValueKind::Overflow, "overflow", "overflow_important")),
        "size" => Ok((StyleValueKind::LayoutSize, "size", "size_important")),
        "min_size" => Ok((StyleValueKind::LayoutSize, "min_size", "min_size_important")),
        "max_size" => Ok((StyleValueKind::LayoutSize, "max_size", "max_size_important")),
        "aspect_ratio" => Ok((
            StyleValueKind::Number,
            "aspect_ratio",
            "aspect_ratio_important",
        )),
        "margin" => Ok((
            StyleValueKind::LengthAutoEdges,
            "margin",
            "margin_important",
        )),
        "gap" => Ok((StyleValueKind::Gap, "gap", "gap_important")),
        "flex_direction" => Ok((
            StyleValueKind::FlexDirection,
            "flex_direction",
            "flex_direction_important",
        )),
        "flex_wrap" => Ok((StyleValueKind::FlexWrap, "flex_wrap", "flex_wrap_important")),
        "flex_basis" => Ok((
            StyleValueKind::Dimension,
            "flex_basis",
            "flex_basis_important",
        )),
        "flex_grow" => Ok((StyleValueKind::Number, "flex_grow", "flex_grow_important")),
        "flex_shrink" => Ok((
            StyleValueKind::Number,
            "flex_shrink",
            "flex_shrink_important",
        )),
        "align_items" => Ok((
            StyleValueKind::AlignItems,
            "align_items",
            "align_items_important",
        )),
        "align_self" => Ok((
            StyleValueKind::AlignSelf,
            "align_self",
            "align_self_important",
        )),
        "align_content" => Ok((
            StyleValueKind::AlignContent,
            "align_content",
            "align_content_important",
        )),
        "justify_items" => Ok((
            StyleValueKind::JustifyItems,
            "justify_items",
            "justify_items_important",
        )),
        "justify_self" => Ok((
            StyleValueKind::JustifySelf,
            "justify_self",
            "justify_self_important",
        )),
        "justify_content" => Ok((
            StyleValueKind::JustifyContent,
            "justify_content",
            "justify_content_important",
        )),
        "grid_auto_flow" => Ok((
            StyleValueKind::GridAutoFlow,
            "grid_auto_flow",
            "grid_auto_flow_important",
        )),
        "grid_template_rows" => Ok((
            StyleValueKind::GridTemplateTracks,
            "grid_template_rows",
            "grid_template_rows_important",
        )),
        "grid_template_columns" => Ok((
            StyleValueKind::GridTemplateTracks,
            "grid_template_columns",
            "grid_template_columns_important",
        )),
        "grid_auto_rows" => Ok((
            StyleValueKind::GridAutoTracks,
            "grid_auto_rows",
            "grid_auto_rows_important",
        )),
        "grid_auto_columns" => Ok((
            StyleValueKind::GridAutoTracks,
            "grid_auto_columns",
            "grid_auto_columns_important",
        )),
        "grid_row" => Ok((StyleValueKind::GridLine, "grid_row", "grid_row_important")),
        "grid_column" => Ok((
            StyleValueKind::GridLine,
            "grid_column",
            "grid_column_important",
        )),
        "position" => Ok((StyleValueKind::Position, "position", "position_important")),
        "inset" => Ok((StyleValueKind::LengthAutoEdges, "inset", "inset_important")),
        "z_index" => Ok((StyleValueKind::ZIndex, "z_index", "z_index_important")),
        _ => Err(Error::new_spanned(
            name,
            "unsupported stylesheet declaration; expected fg, foreground, bg, background, modifier, borders, border_type, padding, image_size, display, box_sizing, overflow, size, min_size, max_size, aspect_ratio, margin, gap, flex_direction, flex_wrap, flex_basis, flex_grow, flex_shrink, align_items, align_self, align_content, justify_items, justify_self, justify_content, grid_auto_flow, grid_template_rows, grid_template_columns, grid_auto_rows, grid_auto_columns, grid_row, grid_column, position, inset, or z_index",
        )),
    }
}
