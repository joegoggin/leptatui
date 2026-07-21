//! Element model for `view!` syntax.
//!
//! This module parses supported XML-like terminal elements and expands them
//! into Leptatui view builder calls.
//!
//! # Modules
//!
//! Expansion is separated into attribute, built-in element, child, custom
//! component, and validation concerns.

mod attributes;
mod builtins;
mod children;
mod custom_component;
mod validation;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Error, Ident, Result, Token,
    parse::{Parse, ParseStream},
};

use crate::view::syntax::{next_is_closing_tag, next_is_self_closing_tag_end};

use self::validation::{is_builtin_element, is_component_name};

use super::{attr::Attr, child::Child};

/// Parsed terminal element with attributes and children.
pub(super) struct Element {
    /// Element tag name, such as `Text` or `Column`.
    pub(super) name: Ident,
    /// Attribute names attached to the element.
    pub(super) attrs: Vec<Attr>,
    /// Child elements or text content nested inside this element.
    pub(super) children: Vec<Child>,
}

impl Parse for Element {
    /// Parses an opening tag, children, and matching closing tag.
    ///
    /// # Arguments
    ///
    /// * `input` — Macro input stream positioned at an opening `<`.
    ///
    /// # Returns
    ///
    /// An [`Element`] containing the tag name, attributes, and children.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the element starts with a closing tag, has
    /// invalid syntax, or closes with a mismatched tag name.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        input.parse::<Token![<]>()?;

        if input.peek(Token![/]) {
            return Err(input.error("view! element cannot start with a closing tag"));
        }

        let name: Ident = input.parse()?;
        let mut attrs = Vec::new();

        while !input.peek(Token![>]) && !next_is_self_closing_tag_end(input) {
            attrs.push(input.parse()?);
        }

        if next_is_self_closing_tag_end(input) {
            input.parse::<Token![/]>()?;
            input.parse::<Token![>]>()?;

            return Ok(Self {
                name,
                attrs,
                children: Vec::new(),
            });
        }

        input.parse::<Token![>]>()?;
        let mut children = Vec::new();
        while !input.is_empty() && !next_is_closing_tag(input) {
            children.push(input.parse()?);
        }

        input.parse::<Token![<]>()?;
        input.parse::<Token![/]>()?;
        let closing_name: Ident = input.parse()?;
        input.parse::<Token![>]>()?;

        if closing_name != name {
            return Err(Error::new_spanned(
                closing_name,
                format!("expected closing tag </{}>", name),
            ));
        }

        Ok(Self {
            name,
            attrs,
            children,
        })
    }
}

impl Element {
    /// Expands this element based on its supported Leptatui tag name.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing view builder calls for this element.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if attributes, children, ancestry, or the element
    /// name are unsupported.
    pub(super) fn expand(&self) -> Result<TokenStream> {
        self.expand_with_parent(None)
    }

    /// Expands this element with its direct semantic parent context.
    ///
    /// # Arguments
    ///
    /// * `parent` — Direct parent tag name, or [`None`] for a root element.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing view builder calls for this element.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if attributes, children, ancestry, or the element
    /// name are unsupported.
    fn expand_with_parent(&self, parent: Option<&str>) -> Result<TokenStream> {
        self.validate_ancestry(parent)?;

        if self.name == "Input" {
            return self.expand_editable_text_control("Input", |value| {
                let leptatui = crate::crate_path::leptatui();
                quote! { #leptatui::input(#value) }
            });
        }

        if self.name == "TextArea" {
            return self.expand_editable_text_control("TextArea", |value| {
                let leptatui = crate::crate_path::leptatui();
                quote! { #leptatui::text_area(#value) }
            });
        }

        if self.name == "Image" {
            return self.expand_image();
        }

        if self.name == "ProgressBar" {
            return self.expand_progress_bar();
        }

        if self.name == "Markdown" {
            return self.expand_markdown();
        }

        match self.name.to_string().as_str() {
            "Block" => self.expand_single_child("Block", |child| {
                let leptatui = crate::crate_path::leptatui();
                quote! { #leptatui::block(#child) }
            }),
            "Row" => self.expand_child_list("Row", |children| {
                let leptatui = crate::crate_path::leptatui();
                quote! { #leptatui::row(::std::vec![#(#leptatui::IntoView::into_view(#children)),*]) }
            }),
            "Column" => self.expand_child_list("Column", |children| {
                let leptatui = crate::crate_path::leptatui();
                quote! { #leptatui::column(::std::vec![#(#leptatui::IntoView::into_view(#children)),*]) }
            }),
            "Form" => self.expand_child_list("Form", |children| {
                let leptatui = crate::crate_path::leptatui();
                quote! { #leptatui::form(::std::vec![#(#leptatui::IntoView::into_view(#children)),*]) }
            }),
            "Text" => self.expand_text_like("Text", |content| {
                let leptatui = crate::crate_path::leptatui();
                quote! { #leptatui::text(#content) }
            }),
            "H1" => self.expand_text_like("H1", |content| {
                let leptatui = crate::crate_path::leptatui();
                quote! { #leptatui::h1(#content) }
            }),
            "H2" => self.expand_text_like("H2", |content| {
                let leptatui = crate::crate_path::leptatui();
                quote! { #leptatui::h2(#content) }
            }),
            "H3" => self.expand_text_like("H3", |content| {
                let leptatui = crate::crate_path::leptatui();
                quote! { #leptatui::h3(#content) }
            }),
            "H4" => self.expand_text_like("H4", |content| {
                let leptatui = crate::crate_path::leptatui();
                quote! { #leptatui::h4(#content) }
            }),
            "H5" => self.expand_text_like("H5", |content| {
                let leptatui = crate::crate_path::leptatui();
                quote! { #leptatui::h5(#content) }
            }),
            "H6" => self.expand_text_like("H6", |content| {
                let leptatui = crate::crate_path::leptatui();
                quote! { #leptatui::h6(#content) }
            }),
            "Paragraph" => self.expand_text_like("Paragraph", |content| {
                let leptatui = crate::crate_path::leptatui();
                quote! { #leptatui::paragraph(#content) }
            }),
            "CodeBlock" => self.expand_text_like("CodeBlock", |content| {
                let leptatui = crate::crate_path::leptatui();
                quote! { #leptatui::code_block(#content) }
            }),
            "OrderedList" => self.expand_element_child_list(
                "OrderedList",
                &["ListItem"],
                |children| {
                    let leptatui = crate::crate_path::leptatui();
                    quote! { #leptatui::ordered_list(::std::vec![#(#leptatui::IntoView::into_view(#children)),*]) }
                },
            ),
            "UnorderedList" => self.expand_element_child_list(
                "UnorderedList",
                &["ListItem"],
                |children| {
                    let leptatui = crate::crate_path::leptatui();
                    quote! { #leptatui::unordered_list(::std::vec![#(#leptatui::IntoView::into_view(#children)),*]) }
                },
            ),
            "ListItem" => self.expand_child_list("ListItem", |children| {
                let leptatui = crate::crate_path::leptatui();
                quote! { #leptatui::list_item(::std::vec![#(#leptatui::IntoView::into_view(#children)),*]) }
            }),
            "Table" => self.expand_element_child_list(
                "Table",
                &["TableHead", "TableBody"],
                |children| {
                    let leptatui = crate::crate_path::leptatui();
                    quote! { #leptatui::table(::std::vec![#(#leptatui::IntoView::into_view(#children)),*]) }
                },
            ),
            "TableHead" => self.expand_element_child_list(
                "TableHead",
                &["TableRow"],
                |children| {
                    let leptatui = crate::crate_path::leptatui();
                    quote! { #leptatui::table_head(::std::vec![#(#leptatui::IntoView::into_view(#children)),*]) }
                },
            ),
            "TableBody" => self.expand_element_child_list(
                "TableBody",
                &["TableRow"],
                |children| {
                    let leptatui = crate::crate_path::leptatui();
                    quote! { #leptatui::table_body(::std::vec![#(#leptatui::IntoView::into_view(#children)),*]) }
                },
            ),
            "TableRow" => self.expand_element_child_list(
                "TableRow",
                &["TableCell"],
                |children| {
                    let leptatui = crate::crate_path::leptatui();
                    quote! { #leptatui::table_row(::std::vec![#(#leptatui::IntoView::into_view(#children)),*]) }
                },
            ),
            "TableCell" => self.expand_text_like("TableCell", |content| {
                let leptatui = crate::crate_path::leptatui();
                quote! { #leptatui::table_cell(#content) }
            }),
            "Button" => self.expand_text_like("Button", |content| {
                let leptatui = crate::crate_path::leptatui();
                quote! { #leptatui::button(#content) }
            }),
            _ if is_component_name(&self.name) => self.expand_component(),
            _ => {
                return Err(Error::new_spanned(
                    &self.name,
                    "unsupported Leptatui element; expected a built-in view tag or a PascalCase component",
                ));
            }
        }
        .and_then(|view| {
            if is_builtin_element(&self.name) {
                let attrs = self.validate_attrs()?;
                self.expand_attrs(view, &attrs)
            } else {
                Ok(view)
            }
        })
    }

    /// Validates the direct parent required by structural document elements.
    ///
    /// # Arguments
    ///
    /// * `parent` — Direct parent tag name, or [`None`] for a root element.
    ///
    /// # Returns
    ///
    /// An empty [`Result`] when the element has valid ancestry.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] when a list item, table section, row, or cell is
    /// used outside its required semantic container.
    fn validate_ancestry(&self, parent: Option<&str>) -> Result<()> {
        let (expected, valid) = match self.name.to_string().as_str() {
            "ListItem" => (
                "OrderedList or UnorderedList",
                matches!(parent, Some("OrderedList" | "UnorderedList")),
            ),
            "TableHead" | "TableBody" => ("Table", parent == Some("Table")),
            "TableRow" => (
                "TableHead or TableBody",
                matches!(parent, Some("TableHead" | "TableBody")),
            ),
            "TableCell" => ("TableRow", parent == Some("TableRow")),
            _ => return Ok(()),
        };

        if valid {
            return Ok(());
        }

        Err(Error::new_spanned(
            &self.name,
            format!("{} must be a direct child of {expected}", self.name),
        ))
    }
}
