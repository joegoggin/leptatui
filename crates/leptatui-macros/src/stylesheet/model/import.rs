//! Imported stylesheet module model for `stylesheet!` syntax.

use std::collections::HashMap;

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{
    Error, Path, Result, Token,
    parse::{Parse, ParseStream},
};

/// Parsed stylesheet module import such as `@use button_mixins;` or
/// `@use button_mixins as button;`.
pub(super) struct UseImport {
    path: Path,
    alias: Ident,
}

impl Parse for UseImport {
    /// Parses a top-level stylesheet module import.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        input.parse::<Token![@]>()?;
        input.parse::<Token![use]>()?;
        let path: Path = input.parse()?;
        let alias = if input.peek(Token![as]) {
            input.parse::<Token![as]>()?;
            input.parse()?
        } else {
            path.segments
                .last()
                .map(|segment| segment.ident.clone())
                .ok_or_else(|| input.error("stylesheet! import requires a module path"))?
        };
        input.parse::<Token![;]>()?;

        Ok(Self { path, alias })
    }
}

impl UseImport {
    /// Returns the local alias used by this import.
    pub(super) fn alias(&self) -> &Ident {
        &self.alias
    }

    /// Expands the import into a typed local binding.
    pub(super) fn expand_binding(
        &self,
        imports: &StylesheetImports,
        leptatui: &TokenStream,
    ) -> Result<TokenStream> {
        let binding = imports.get(&self.alias)?;
        let path = &self.path;

        Ok(quote! {
            let #binding: #leptatui::StyleModule = #path();
        })
    }
}

/// Compile-time stylesheet module import lookup.
#[derive(Default)]
pub(super) struct StylesheetImports {
    values: HashMap<String, Ident>,
}

impl StylesheetImports {
    /// Adds one parsed import alias.
    pub(super) fn insert(&mut self, import: &UseImport) -> Result<()> {
        let alias = import.alias().to_string();

        if self.values.contains_key(&alias) {
            return Err(Error::new_spanned(
                import.alias(),
                format!("duplicate stylesheet module alias `{alias}`"),
            ));
        }

        self.values.insert(alias, binding_ident(import.alias()));
        Ok(())
    }

    /// Looks up the generated binding for an import alias.
    pub(super) fn get(&self, alias: &Ident) -> Result<&Ident> {
        self.values.get(&alias.to_string()).ok_or_else(|| {
            Error::new_spanned(alias, format!("unknown stylesheet module alias `{alias}`"))
        })
    }
}

/// Creates the generated local binding identifier for an import alias.
fn binding_ident(alias: &Ident) -> Ident {
    format_ident!("__leptatui_style_module_{}", alias, span = alias.span())
}

/// Returns whether the stream starts with a top-level module import.
pub(super) fn starts_use(input: ParseStream<'_>) -> bool {
    let fork = input.fork();
    fork.parse::<Token![@]>().is_ok() && fork.parse::<Token![use]>().is_ok()
}
