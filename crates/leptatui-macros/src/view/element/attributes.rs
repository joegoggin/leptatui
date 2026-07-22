//! Element attribute validation and builder-method expansion.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Result};

use super::{
    Element,
    validation::{AttrKind, ValidatedAttr, reject_literal_callback, reject_literal_typed_attr},
};

impl Element {
    /// Validates and classifies every attribute name accepted by `view!`.
    ///
    /// # Returns
    ///
    /// Attribute references paired with their accepted kinds.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if an attribute is unsupported for the element.
    pub(super) fn validate_attrs(&self) -> Result<Vec<ValidatedAttr<'_>>> {
        let element_name = self.name.to_string();
        let mut attrs = Vec::with_capacity(self.attrs.len());

        for attr in &self.attrs {
            let kind = match attr.name.to_string().as_str() {
                "class" => AttrKind::Class,
                "id" => AttrKind::Id,
                "style" => AttrKind::Style,
                "src" if element_name == "Image" => AttrKind::ImageSource,
                "src" if element_name == "Markdown" => AttrKind::MarkdownSrc,
                "src" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! src attribute is only supported on Image or Markdown",
                    ));
                }
                "href" if element_name == "Link" => AttrKind::Href,
                "href" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! href attribute is only supported on Link",
                    ));
                }
                "alt" if element_name == "Image" => AttrKind::Alt,
                "alt" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! alt attribute is only supported on Image",
                    ));
                }
                "value" if matches!(element_name.as_str(), "Input" | "TextArea") => {
                    AttrKind::InputValue
                }
                "value" if element_name == "ProgressBar" => AttrKind::ProgressValue,
                "value" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! value attribute is only supported on Input, TextArea, or ProgressBar",
                    ));
                }
                "label" if element_name == "ProgressBar" => AttrKind::Label,
                "label" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! label attribute is only supported on ProgressBar",
                    ));
                }
                "placeholder" if matches!(element_name.as_str(), "Input" | "TextArea") => {
                    AttrKind::Placeholder
                }
                "placeholder" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! placeholder attribute is only supported on Input or TextArea",
                    ));
                }
                "on_press" if element_name == "Button" => AttrKind::OnPress,
                "on_press" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! on_press attribute is only supported on Button",
                    ));
                }
                "on_submit" if element_name == "Form" => AttrKind::OnSubmit,
                "on_submit" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! on_submit attribute is only supported on Form",
                    ));
                }
                "on_cancel" if element_name == "Form" => AttrKind::OnCancel,
                "on_cancel" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! on_cancel attribute is only supported on Form",
                    ));
                }
                "on_input" if matches!(element_name.as_str(), "Input" | "TextArea") => {
                    AttrKind::OnInput
                }
                "on_input" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! on_input attribute is only supported on Input or TextArea",
                    ));
                }
                "start" if element_name == "OrderedList" => AttrKind::Start,
                "start" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! start attribute is only supported on OrderedList",
                    ));
                }
                "alignment" if element_name == "TableCell" => AttrKind::Alignment,
                "alignment" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! alignment attribute is only supported on TableCell",
                    ));
                }
                "language" if element_name == "CodeBlock" => AttrKind::Language,
                "language" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! language attribute is only supported on CodeBlock",
                    ));
                }
                "line_numbers" if matches!(element_name.as_str(), "CodeBlock" | "Markdown") => {
                    AttrKind::LineNumbers
                }
                "line_numbers" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! line_numbers attribute is only supported on CodeBlock or Markdown",
                    ));
                }
                "syntax_theme" if matches!(element_name.as_str(), "CodeBlock" | "Markdown") => {
                    AttrKind::SyntaxTheme
                }
                "syntax_theme" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! syntax_theme attribute is only supported on CodeBlock or Markdown",
                    ));
                }
                _ => {
                    let message = match element_name.as_str() {
                        "Button" => {
                            "unsupported view! attribute; expected class, id, style, or on_press"
                        }
                        "Link" => "unsupported view! attribute; expected class, id, style, or href",
                        "Form" => {
                            "unsupported view! attribute; expected class, id, style, on_submit, or on_cancel"
                        }
                        "Input" | "TextArea" => {
                            "unsupported view! attribute; expected class, id, style, value, placeholder, or on_input"
                        }
                        "Image" => {
                            "unsupported view! attribute; expected class, id, style, src, or alt"
                        }
                        "ProgressBar" => {
                            "unsupported view! attribute; expected class, id, style, value, or label"
                        }
                        "Markdown" => {
                            "unsupported view! attribute; expected class, id, style, src, line_numbers, or syntax_theme"
                        }
                        "OrderedList" => {
                            "unsupported view! attribute; expected class, id, style, or start"
                        }
                        "TableCell" => {
                            "unsupported view! attribute; expected class, id, style, or alignment"
                        }
                        "CodeBlock" => {
                            "unsupported view! attribute; expected class, id, style, language, line_numbers, or syntax_theme"
                        }
                        _ => "unsupported view! attribute; expected class, id, or style",
                    };
                    return Err(Error::new_spanned(&attr.name, message));
                }
            };

            attrs.push(ValidatedAttr { attr, kind });
        }

        Ok(attrs)
    }

    /// Expands supported attributes into selector metadata setters.
    ///
    /// # Arguments
    ///
    /// * `view` — Already-expanded view builder expression.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the view expression wrapped with metadata
    /// setters.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] for duplicate attributes, invalid callback
    /// literals, string literals used for typed configuration, or unbraced
    /// values where braces are required.
    pub(super) fn expand_attrs(
        &self,
        view: TokenStream,
        attrs: &[ValidatedAttr<'_>],
    ) -> Result<TokenStream> {
        for (index, current) in attrs.iter().enumerate() {
            if let Some(duplicate) = attrs[index + 1..]
                .iter()
                .find(|candidate| candidate.kind == current.kind)
            {
                return Err(Error::new_spanned(
                    &duplicate.attr.name,
                    format!(
                        "{} expects at most one {} attribute",
                        self.name, duplicate.attr.name
                    ),
                ));
            }
        }

        let mut expanded = view;

        for ValidatedAttr { attr, kind } in attrs {
            let value = attr.value.to_tokens();
            expanded = match kind {
                AttrKind::Class => quote! { (#expanded).with_classes(#value) },
                AttrKind::Id => quote! { (#expanded).with_id(#value) },
                AttrKind::Style => {
                    if attr.value.is_literal() {
                        return Err(Error::new_spanned(
                            &attr.name,
                            "view! style attribute must be a TuiStyle expression",
                        ));
                    }

                    quote! { (#expanded).with_inline_style(#value) }
                }
                AttrKind::OnPress => {
                    reject_literal_callback(attr, "on_press")?;
                    quote! { (#expanded).on_press(#value) }
                }
                AttrKind::OnSubmit => {
                    reject_literal_callback(attr, "on_submit")?;
                    quote! { (#expanded).on_submit(#value) }
                }
                AttrKind::OnCancel => {
                    reject_literal_callback(attr, "on_cancel")?;
                    quote! { (#expanded).on_cancel(#value) }
                }
                AttrKind::InputValue => expanded,
                AttrKind::ImageSource => expanded,
                AttrKind::ProgressValue => expanded,
                AttrKind::MarkdownSrc => expanded,
                AttrKind::Href => expanded,
                AttrKind::Placeholder => quote! { (#expanded).placeholder(#value) },
                AttrKind::Alt => quote! { (#expanded).alt(#value) },
                AttrKind::Label => quote! { (#expanded).label(#value) },
                AttrKind::Start => {
                    reject_literal_typed_attr(attr, "start", "usize")?;
                    quote! { (#expanded).start(#value) }
                }
                AttrKind::Alignment => {
                    reject_literal_typed_attr(attr, "alignment", "CellAlignment")?;
                    quote! { (#expanded).alignment(#value) }
                }
                AttrKind::Language => quote! { (#expanded).language(#value) },
                AttrKind::LineNumbers if self.name == "Markdown" => {
                    reject_literal_typed_attr(attr, "line_numbers", "bool")?;
                    expanded
                }
                AttrKind::LineNumbers => {
                    reject_literal_typed_attr(attr, "line_numbers", "bool")?;
                    quote! { (#expanded).line_numbers(#value) }
                }
                AttrKind::SyntaxTheme if self.name == "Markdown" => {
                    reject_literal_typed_attr(attr, "syntax_theme", "SyntaxTheme")?;
                    expanded
                }
                AttrKind::SyntaxTheme => {
                    reject_literal_typed_attr(attr, "syntax_theme", "SyntaxTheme")?;
                    quote! { (#expanded).syntax_theme(#value) }
                }
                AttrKind::OnInput => {
                    reject_literal_callback(attr, "on_input")?;
                    quote! { (#expanded).on_input(#value) }
                }
            };

            if !matches!(
                *kind,
                AttrKind::OnPress
                    | AttrKind::OnSubmit
                    | AttrKind::OnCancel
                    | AttrKind::OnInput
                    | AttrKind::InputValue
                    | AttrKind::ImageSource
                    | AttrKind::ProgressValue
                    | AttrKind::MarkdownSrc
                    | AttrKind::Href
                    | AttrKind::Style
                    | AttrKind::Start
                    | AttrKind::Alignment
                    | AttrKind::Language
                    | AttrKind::LineNumbers
                    | AttrKind::SyntaxTheme
            ) && attr.value.is_unbraced_expr()
            {
                return Err(Error::new_spanned(
                    &attr.name,
                    "view! attribute values must be string literals or braced expressions",
                ));
            }
        }

        Ok(expanded)
    }
}
