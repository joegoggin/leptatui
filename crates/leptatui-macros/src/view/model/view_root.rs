use proc_macro2::TokenStream;
use syn::{
    Result,
    parse::{Parse, ParseStream},
};

use super::element::Element;

/// Root node for a `view!` invocation.
pub(in crate::view) struct ViewRoot {
    /// Single root element required by the macro.
    pub(super) element: Element,
}

impl Parse for ViewRoot {
    /// Parses a `view!` invocation with exactly one root element.
    ///
    /// # Arguments
    ///
    /// * `input` - Macro input stream to parse.
    ///
    /// # Returns
    ///
    /// A [`ViewRoot`] containing the parsed root element.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if parsing fails or tokens remain after the root
    /// element.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let element = input.parse()?;

        if !input.is_empty() {
            return Err(input.error("view! expects a single root element"));
        }

        Ok(Self { element })
    }
}

impl ViewRoot {
    /// Expands the root element into generated node code.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the expanded root node.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the root element is unsupported or malformed.
    pub(in crate::view) fn expand(self) -> Result<TokenStream> {
        self.element.expand()
    }
}
