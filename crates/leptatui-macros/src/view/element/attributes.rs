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
                "editable" if element_name == "Markdown" => AttrKind::Editable,
                "editable" => {
                    return Err(Error::new_spanned(
                        &attr.name,
                        "view! editable attribute is only supported on Markdown",
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
                            "unsupported view! attribute; expected class, id, style, src, line_numbers, or editable"
                        }
                        "OrderedList" => {
                            "unsupported view! attribute; expected class, id, style, or start"
                        }
                        "TableCell" => {
                            "unsupported view! attribute; expected class, id, style, or alignment"
                        }
                        "CodeBlock" => {
                            "unsupported view! attribute; expected class, id, style, language, or line_numbers"
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
        self.expand_attrs_with_tracking(view, attrs, true)
    }

    /// Expands concrete semantic attributes inside an already tracked parent.
    ///
    /// # Arguments
    ///
    /// * `view` — Concrete semantic node.
    /// * `attrs` — Validated attributes applied directly to that node.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] of fluent setters without nested binding boundaries.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] for invalid or duplicate attributes.
    pub(super) fn expand_static_attrs(
        &self,
        view: TokenStream,
        attrs: &[ValidatedAttr<'_>],
    ) -> Result<TokenStream> {
        self.expand_attrs_with_tracking(view, attrs, false)
    }

    /// Applies fluent setters using either local bindings or parent tracking.
    ///
    /// # Arguments
    ///
    /// * `view` — Expanded node constructor.
    /// * `attrs` — Validated attributes.
    /// * `track` — Whether expressions need their own binding boundary.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the configured node.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] for malformed attributes.
    fn expand_attrs_with_tracking(
        &self,
        view: TokenStream,
        attrs: &[ValidatedAttr<'_>],
        track: bool,
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
        let mut binding = quote! { __leptatui_view };
        let mut has_binding = false;
        let mut captured_expressions = Vec::new();

        for ValidatedAttr { attr, kind } in attrs {
            let reactive = track
                && !attr.value.is_literal()
                && !matches!(
                    kind,
                    AttrKind::OnPress
                        | AttrKind::OnSubmit
                        | AttrKind::OnCancel
                        | AttrKind::OnInput
                        | AttrKind::Href
                        | AttrKind::MarkdownSrc
                        | AttrKind::Editable
                )
                && !(self.name == "Markdown" && *kind == AttrKind::LineNumbers);
            if reactive {
                captured_expressions.push(syn::parse2::<syn::Expr>(attr.value.to_tokens())?);
            }
            let value = if reactive || !track {
                attr.value.repeat_tokens()
            } else {
                attr.value.to_tokens()
            };
            let target = if reactive {
                binding.clone()
            } else {
                expanded.clone()
            };
            let next = match kind {
                AttrKind::Class => quote! { (#target).with_classes(#value) },
                AttrKind::Id => quote! { (#target).with_id(#value) },
                AttrKind::Style => {
                    if attr.value.is_literal() {
                        return Err(Error::new_spanned(
                            &attr.name,
                            "view! style attribute must be a TuiStyle expression",
                        ));
                    }

                    quote! { (#target).with_inline_style(#value) }
                }
                AttrKind::OnPress => {
                    reject_literal_callback(attr, "on_press")?;
                    quote! { (#target).on_press(#value) }
                }
                AttrKind::OnSubmit => {
                    reject_literal_callback(attr, "on_submit")?;
                    quote! { (#target).on_submit(#value) }
                }
                AttrKind::OnCancel => {
                    reject_literal_callback(attr, "on_cancel")?;
                    quote! { (#target).on_cancel(#value) }
                }
                AttrKind::InputValue if reactive => {
                    quote! { (#target).__value(#value, __leptatui_initial) }
                }
                AttrKind::InputValue => target,
                AttrKind::ImageSource if reactive => {
                    quote! { (#target).__value(#value, __leptatui_initial) }
                }
                AttrKind::ImageSource => target,
                AttrKind::ProgressValue if reactive => {
                    quote! { (#target).__value(#value, __leptatui_initial) }
                }
                AttrKind::ProgressValue => target,
                AttrKind::MarkdownSrc => target,
                AttrKind::Href => target,
                AttrKind::Placeholder => quote! { (#target).placeholder(#value) },
                AttrKind::Alt => quote! { (#target).alt(#value) },
                AttrKind::Label => quote! { (#target).label(#value) },
                AttrKind::Start => {
                    reject_literal_typed_attr(attr, "start", "usize")?;
                    quote! { (#target).start(#value) }
                }
                AttrKind::Alignment => {
                    reject_literal_typed_attr(attr, "alignment", "CellAlignment")?;
                    quote! { (#target).alignment(#value) }
                }
                AttrKind::Language => quote! { (#target).language(#value) },
                AttrKind::LineNumbers if self.name == "Markdown" => {
                    reject_literal_typed_attr(attr, "line_numbers", "bool")?;
                    target
                }
                AttrKind::LineNumbers => {
                    reject_literal_typed_attr(attr, "line_numbers", "bool")?;
                    quote! { (#target).line_numbers(#value) }
                }
                AttrKind::Editable => {
                    reject_literal_typed_attr(attr, "editable", "bool")?;
                    target
                }
                AttrKind::OnInput => {
                    reject_literal_callback(attr, "on_input")?;
                    quote! { (#target).on_input(#value) }
                }
            };
            if reactive {
                binding = next;
                has_binding = true;
            } else {
                expanded = next;
            }

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
                    | AttrKind::Editable
            ) && attr.value.is_unbraced_expr()
            {
                return Err(Error::new_spanned(
                    &attr.name,
                    "view! attribute values must be string literals or braced expressions",
                ));
            }
        }

        if has_binding {
            let leptatui = crate::crate_path::leptatui();
            let captures = crate::view::captures::capture_values(&captured_expressions);
            Ok(quote! {{
                let __leptatui_node = #expanded;
                #captures
                #leptatui::__private::__bind(__leptatui_node, move |__leptatui_view, __leptatui_initial| #binding)
            }})
        } else {
            Ok(expanded)
        }
    }
}
