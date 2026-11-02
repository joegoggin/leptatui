//! Reactive tables retaining the concrete semantic row and cell tree.
//!
//! Table measurement reads concrete cell content directly. A single outer
//! boundary tracks all table expressions and reconciles the concrete table
//! instead of introducing structural wrappers between its sections and cells.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Expr, Result};

use super::Element;
use crate::view::{child::Child, text_content::TextContent};

impl Element {
    /// Expands a complete table under one automatically tracked boundary.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] constructing a reactive semantic table.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if table ancestry, children, or attributes are invalid.
    pub(super) fn expand_reactive_table(&self) -> Result<TokenStream> {
        let leptatui = crate::crate_path::leptatui();
        let mut expressions = Vec::new();
        let table = self.expand_table_node(None, &mut expressions)?;
        let captures = crate::view::captures::capture_values(&expressions);
        Ok(quote! {{ #captures #leptatui::dynamic(move || #table) }})
    }

    /// Expands concrete table nodes and collects expressions tracked by their parent.
    ///
    /// # Arguments
    ///
    /// * `parent` — Semantic parent used for ancestry diagnostics.
    /// * `expressions` — Data expressions whose owned captures must be retained.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] constructing one concrete semantic node.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the node has invalid ancestry, children, or attributes.
    fn expand_table_node(
        &self,
        parent: Option<&str>,
        expressions: &mut Vec<Expr>,
    ) -> Result<TokenStream> {
        self.validate_ancestry(parent)?;
        let leptatui = crate::crate_path::leptatui();
        let name = self.name.to_string();
        let view = if name == "TableCell" {
            if self.children.len() != 1 {
                return Err(Error::new_spanned(
                    &self.name,
                    "TableCell expects exactly one text child",
                ));
            }
            let Child::Text(content) = &self.children[0] else {
                return Err(Error::new_spanned(
                    &self.name,
                    "TableCell expects text content, not child elements",
                ));
            };
            let content = match content {
                TextContent::Literal(value) => quote! { #value },
                TextContent::Expr(expr) => {
                    expressions.push(*expr.clone());
                    if matches!(expr.as_ref(), Expr::Path(_)) {
                        quote! { (#expr).clone() }
                    } else {
                        quote! { #expr }
                    }
                }
            };
            quote! { #leptatui::table_cell(#leptatui::__private::__read_rich_text(#content)) }
        } else {
            let allowed = match name.as_str() {
                "Table" => &["TableHead", "TableBody"][..],
                "TableHead" | "TableBody" => &["TableRow"][..],
                "TableRow" => &["TableCell"][..],
                _ => unreachable!("validated table node"),
            };
            if self.children.is_empty() {
                return Err(Error::new_spanned(
                    &self.name,
                    format!("{name} expects at least one child element"),
                ));
            }
            let mut children = Vec::new();
            for child in &self.children {
                let Child::Element(child) = child else {
                    return Err(Error::new_spanned(
                        &self.name,
                        format!("{name} only accepts {} children", allowed.join(" or ")),
                    ));
                };
                if !allowed.iter().any(|allowed| child.name == *allowed) {
                    return Err(Error::new_spanned(
                        &child.name,
                        format!("{name} only accepts {} children", allowed.join(" or ")),
                    ));
                }
                children.push(child.expand_table_node(Some(&name), expressions)?);
            }
            let builder = match name.as_str() {
                "Table" => quote! { #leptatui::table },
                "TableHead" => quote! { #leptatui::table_head },
                "TableBody" => quote! { #leptatui::table_body },
                "TableRow" => quote! { #leptatui::table_row },
                _ => unreachable!(),
            };
            quote! { #builder(::std::vec![#(#leptatui::IntoView::into_view(#children)),*]) }
        };
        let attrs = self.validate_attrs()?;
        for attr in &self.attrs {
            if !attr.value.is_literal() {
                expressions.push(syn::parse2(attr.value.to_tokens())?);
            }
        }
        self.expand_static_attrs(view, &attrs)
    }
}
