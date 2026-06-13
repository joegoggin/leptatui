//! Variable model for `stylesheet!` syntax.
//!
//! This module parses variable definitions, parses variable references, and
//! stores compile-time references to variable expressions for expansion.

use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Error, Expr, Ident, Result, Token,
    parse::{Parse, ParseStream},
};

/// Parsed variable definition such as `$primary: Color::Blue;`.
pub(super) struct Variable {
    /// Variable identifier without the `$` prefix.
    name: Ident,
    /// Rust expression assigned to this variable.
    value: Expr,
}

impl Variable {
    /// Returns this variable's identifier without the `$` prefix.
    pub(super) fn name(&self) -> &Ident {
        &self.name
    }

    /// Returns the Rust expression assigned to this variable.
    pub(super) fn value(&self) -> &Expr {
        &self.value
    }
}

impl Parse for Variable {
    /// Parses a stylesheet variable definition.
    ///
    /// # Arguments
    ///
    /// * `input` — Macro input stream positioned at a variable definition.
    ///
    /// # Returns
    ///
    /// A [`Variable`] containing the parsed identifier and value expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the definition is missing `$`, `:`, a value
    /// expression, or a trailing semicolon.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        input.parse::<Token![$]>()?;
        let name = input.parse()?;
        input.parse::<Token![:]>()?;
        let value = input.parse()?;
        input.parse::<Token![;]>()?;

        Ok(Self { name, value })
    }
}

/// Parsed variable reference such as `$primary`.
pub(super) struct VariableRef {
    /// Referenced variable identifier without the `$` prefix.
    name: Ident,
}

impl Parse for VariableRef {
    /// Parses a stylesheet variable reference.
    ///
    /// # Arguments
    ///
    /// * `input` — Macro input stream positioned at a variable reference.
    ///
    /// # Returns
    ///
    /// A [`VariableRef`] containing the referenced identifier.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the reference is missing `$` or an identifier.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        input.parse::<Token![$]>()?;
        Ok(Self {
            name: input.parse()?,
        })
    }
}

/// Parsed imported variable reference such as `colors.$fg`.
pub(super) struct ImportedVariableRef {
    /// Imported module alias.
    alias: Ident,
    /// Referenced variable identifier without the `$` prefix.
    name: Ident,
}

impl Parse for ImportedVariableRef {
    /// Parses an imported stylesheet variable reference.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let alias = input.parse()?;
        input.parse::<Token![.]>()?;
        input.parse::<Token![$]>()?;
        let name = input.parse()?;

        Ok(Self { alias, name })
    }
}

impl ImportedVariableRef {
    /// Returns the imported module alias.
    pub(super) fn alias(&self) -> &Ident {
        &self.alias
    }

    /// Returns the referenced variable identifier.
    pub(super) fn name(&self) -> &Ident {
        &self.name
    }
}

impl VariableRef {
    /// Expands this reference to the variable's stored Rust expression.
    ///
    /// # Arguments
    ///
    /// * `variables` — Stylesheet variables available to this reference.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the referenced variable expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if this reference names an unknown stylesheet
    /// variable.
    pub(super) fn expand(&self, variables: &StylesheetVariables<'_>) -> Result<TokenStream> {
        let Some(value) = variables.get(&self.name) else {
            return Err(Error::new_spanned(
                &self.name,
                format!("unknown stylesheet variable `${}`", self.name),
            ));
        };

        Ok(quote! { #value })
    }
}

/// Returns whether the stream starts with an imported variable reference.
pub(super) fn starts_imported_variable(input: ParseStream<'_>) -> bool {
    let fork = input.fork();

    fork.parse::<Ident>().is_ok()
        && fork.parse::<Token![.]>().is_ok()
        && fork.parse::<Token![$]>().is_ok()
}

/// Compile-time stylesheet variable lookup.
#[derive(Default)]
pub(super) struct StylesheetVariables<'a> {
    /// Variable expressions keyed by their source identifier.
    values: HashMap<String, &'a Expr>,
}

impl<'a> StylesheetVariables<'a> {
    /// Adds one parsed variable definition.
    ///
    /// # Arguments
    ///
    /// * `variable` — Parsed variable definition to store.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if a variable with the same name was already
    /// stored.
    pub(super) fn insert(&mut self, variable: &'a Variable) -> Result<()> {
        let name = variable.name.to_string();

        if self.values.contains_key(&name) {
            return Err(Error::new_spanned(
                &variable.name,
                format!("duplicate stylesheet variable `${name}`"),
            ));
        }

        self.values.insert(name, &variable.value);
        Ok(())
    }

    /// Looks up a variable expression by identifier.
    ///
    /// # Arguments
    ///
    /// * `name` — Variable identifier to look up.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the stored expression when the variable exists.
    pub(super) fn get(&self, name: &Ident) -> Option<&'a Expr> {
        self.values.get(&name.to_string()).copied()
    }
}
