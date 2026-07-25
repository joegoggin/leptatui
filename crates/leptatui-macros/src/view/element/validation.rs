//! Shared element and attribute validation helpers.

use syn::{Error, Ident, Result};

use crate::view::attr::Attr;

/// Rejects a literal callback attribute value.
///
/// # Arguments
///
/// * `attr` — Parsed callback attribute to inspect.
/// * `attribute_name` — User-facing callback attribute name for diagnostics.
///
/// # Returns
///
/// An empty [`Result`] when the attribute value is not a literal.
///
/// # Errors
///
/// Returns [`syn::Error`] if the callback value is a literal.
pub(super) fn reject_literal_callback(attr: &Attr, attribute_name: &str) -> Result<()> {
    if attr.value.is_literal() {
        return Err(Error::new_spanned(
            &attr.name,
            format!("view! {attribute_name} attribute must be a callback expression"),
        ));
    }

    Ok(())
}

/// Rejects a string literal for an attribute requiring a typed expression.
///
/// # Arguments
///
/// * `attr` — Parsed typed attribute to inspect.
/// * `attribute_name` — User-facing attribute name for diagnostics.
/// * `expected_type` — Rust type required by the corresponding builder.
///
/// # Returns
///
/// An empty [`Result`] when the value is an expression.
///
/// # Errors
///
/// Returns [`syn::Error`] if the value is a string literal.
pub(super) fn reject_literal_typed_attr(
    attr: &Attr,
    attribute_name: &str,
    expected_type: &str,
) -> Result<()> {
    if attr.value.is_literal() {
        return Err(Error::new_spanned(
            &attr.name,
            format!("view! {attribute_name} attribute must be a {expected_type} expression"),
        ));
    }

    Ok(())
}

/// Returns whether an identifier is one of the built-in `view!` elements.
pub(super) fn is_builtin_element(name: &Ident) -> bool {
    matches!(
        name.to_string().as_str(),
        "Block"
            | "Div"
            | "Form"
            | "Text"
            | "H1"
            | "H2"
            | "H3"
            | "H4"
            | "H5"
            | "H6"
            | "Paragraph"
            | "Link"
            | "Markdown"
            | "CodeBlock"
            | "OrderedList"
            | "UnorderedList"
            | "ListItem"
            | "Table"
            | "TableHead"
            | "TableBody"
            | "TableRow"
            | "TableCell"
            | "Button"
            | "Input"
            | "TextArea"
            | "Image"
            | "ProgressBar"
    )
}

/// Returns whether an identifier should be treated as a component tag.
pub(super) fn is_component_name(name: &Ident) -> bool {
    name.to_string()
        .chars()
        .next()
        .is_some_and(char::is_uppercase)
}

/// Internal attribute validation output for `Element` expansion.
mod attr_validation {
    use super::Attr;

    /// Supported `view!` attribute kinds after validation.
    #[derive(Clone, Copy, Eq, PartialEq)]
    pub(in crate::view::element) enum AttrKind {
        /// `class` selector metadata.
        Class,
        /// `id` selector metadata.
        Id,
        /// Inline `style` override.
        Style,
        /// Required input value.
        InputValue,
        /// Required image source.
        ImageSource,
        /// Required progress value.
        ProgressValue,
        /// Required Markdown file path.
        MarkdownSrc,
        /// Required standalone link destination.
        Href,
        /// Input placeholder text.
        Placeholder,
        /// Image fallback text.
        Alt,
        /// Progress bar label text.
        Label,
        /// Ordered-list starting marker.
        Start,
        /// Table-cell horizontal alignment.
        Alignment,
        /// Code-block syntax language.
        Language,
        /// Code-block line-number visibility.
        LineNumbers,
        /// Code-block highlighting theme.
        SyntaxTheme,
        /// Button activation callback.
        OnPress,
        /// Form submit callback.
        OnSubmit,
        /// Form cancel callback.
        OnCancel,
        /// Input value-change callback.
        OnInput,
    }

    /// Attribute paired with its validated kind.
    pub(in crate::view::element) struct ValidatedAttr<'a> {
        /// Parsed source attribute.
        pub(in crate::view::element) attr: &'a Attr,
        /// Accepted attribute kind.
        pub(in crate::view::element) kind: AttrKind,
    }
}

pub(super) use attr_validation::{AttrKind, ValidatedAttr};
