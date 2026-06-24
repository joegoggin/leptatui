//! Reusable declaration mixin model for `stylesheet!` syntax.
//!
//! This module parses top-level `@mixin` definitions, parses rule-local
//! `@include` references, and stores compile-time mixin definitions for
//! expansion into ordinary style declarations.

use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Error, Ident, Result, Token, braced,
    parse::{Parse, ParseStream},
};

use crate::stylesheet::model::{
    declaration::Declaration, import::StylesheetImports, variable::StylesheetVariables,
};

mod kw {
    syn::custom_keyword!(mixin);
    syn::custom_keyword!(include);
}

/// Parsed reusable declaration group such as `@mixin panel { ... }`.
pub(super) struct Mixin {
    /// Mixin identifier used by `@include`.
    name: Ident,
    /// Declarations and includes expanded wherever the mixin is included.
    items: Vec<MixinItem>,
}

/// Ordered style item inside a mixin body.
enum MixinItem {
    /// Ordinary style declaration.
    Declaration(Box<Declaration>),
    /// Reusable declaration mixin include.
    MixinInclude(MixinInclude),
}

impl MixinItem {
    /// Applies this mixin item to an in-progress declaration expression.
    fn expand(
        &self,
        style: TokenStream,
        variables: &StylesheetVariables<'_>,
        imports: &StylesheetImports,
        mixins: &StylesheetMixins<'_>,
        stack: &mut Vec<String>,
    ) -> Result<TokenStream> {
        match self {
            Self::Declaration(declaration) => declaration.expand(style, variables, imports),
            Self::MixinInclude(include) => {
                include.expand_with_stack(style, variables, imports, mixins, stack)
            }
        }
    }
}

impl Parse for Mixin {
    /// Parses a stylesheet mixin definition.
    ///
    /// # Arguments
    ///
    /// * `input` — Macro input stream positioned at a mixin definition.
    ///
    /// # Returns
    ///
    /// A [`Mixin`] containing the parsed identifier and declarations.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the definition is missing `@mixin`, a name, a
    /// braced declaration block, comma-separated declarations, or at least one
    /// declaration.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        input.parse::<Token![@]>()?;
        input.parse::<kw::mixin>()?;
        let name = input.parse()?;

        let content;
        braced!(content in input);

        let mut items = Vec::new();
        while !content.is_empty() {
            if content.peek(Token![@]) {
                items.push(MixinItem::MixinInclude(content.parse()?));
            } else {
                items.push(MixinItem::Declaration(Box::new(content.parse()?)));
            }

            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            } else if !content.is_empty() {
                return Err(
                    content.error("stylesheet! mixin declarations must be separated by commas")
                );
            }
        }

        if items.is_empty() {
            return Err(
                content.error("stylesheet! mixin requires at least one declaration or include")
            );
        }

        Ok(Self { name, items })
    }
}

impl Mixin {
    /// Returns this mixin's identifier.
    ///
    /// # Returns
    ///
    /// An [`struct@Ident`] reference for this mixin's source name.
    pub(super) fn name(&self) -> &Ident {
        &self.name
    }

    /// Applies this mixin's items to an in-progress style expression.
    ///
    /// # Arguments
    ///
    /// * `style` — Existing `TuiStyle` expression to wrap with this mixin's
    ///   declarations.
    /// * `variables` — Stylesheet variables available to declaration values.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the updated style expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if a declaration name is unsupported or a
    /// referenced stylesheet variable is unknown.
    pub(super) fn expand(
        &self,
        style: TokenStream,
        variables: &StylesheetVariables<'_>,
        imports: &StylesheetImports,
        mixins: &StylesheetMixins<'_>,
    ) -> Result<TokenStream> {
        self.expand_with_stack(style, variables, imports, mixins, &mut Vec::new())
    }

    /// Applies this mixin while tracking local include recursion.
    fn expand_with_stack(
        &self,
        mut style: TokenStream,
        variables: &StylesheetVariables<'_>,
        imports: &StylesheetImports,
        mixins: &StylesheetMixins<'_>,
        stack: &mut Vec<String>,
    ) -> Result<TokenStream> {
        let name = self.name.to_string();

        if let Some(position) = stack.iter().position(|value| value == &name) {
            let mut cycle = stack[position..].to_vec();
            cycle.push(name);

            return Err(Error::new_spanned(
                &self.name,
                format!("cyclic stylesheet mixin include `{}`", cycle.join(" -> ")),
            ));
        }

        stack.push(name);

        for item in &self.items {
            style = item.expand(style, variables, imports, mixins, stack)?;
        }

        stack.pop();
        Ok(style)
    }
}

/// Parsed mixin include such as `@include panel`.
pub(super) struct MixinInclude {
    /// Referenced local or imported mixin.
    reference: MixinReference,
}

/// Local or imported mixin reference.
enum MixinReference {
    /// Mixin defined in the current `stylesheet!` invocation.
    Local(Ident),
    /// Mixin exported by an imported style module.
    Imported { alias: Ident, name: Ident },
}

impl Parse for MixinInclude {
    /// Parses a stylesheet mixin include.
    ///
    /// # Arguments
    ///
    /// * `input` — Macro input stream positioned at an include.
    ///
    /// # Returns
    ///
    /// A [`MixinInclude`] containing the referenced mixin identifier.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the include is missing `@include` or a mixin
    /// name.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        input.parse::<Token![@]>()?;
        input.parse::<kw::include>()?;

        let first = input.parse()?;
        let reference = if input.peek(Token![.]) {
            input.parse::<Token![.]>()?;

            MixinReference::Imported {
                alias: first,
                name: input.parse()?,
            }
        } else {
            MixinReference::Local(first)
        };

        Ok(Self { reference })
    }
}

impl MixinInclude {
    /// Expands this include into the referenced mixin's declarations.
    ///
    /// # Arguments
    ///
    /// * `style` — Existing `TuiStyle` expression to wrap with the referenced
    ///   mixin's declarations.
    /// * `variables` — Stylesheet variables available to declaration values.
    /// * `mixins` — Stylesheet mixins available to this include.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the updated style expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if this include names an unknown stylesheet mixin,
    /// a declaration name is unsupported, or a referenced stylesheet variable is
    /// unknown.
    pub(super) fn expand(
        &self,
        style: TokenStream,
        variables: &StylesheetVariables<'_>,
        imports: &StylesheetImports,
        mixins: &StylesheetMixins<'_>,
    ) -> Result<TokenStream> {
        self.expand_with_stack(style, variables, imports, mixins, &mut Vec::new())
    }

    /// Expands this include while preserving the active local include stack.
    fn expand_with_stack(
        &self,
        style: TokenStream,
        variables: &StylesheetVariables<'_>,
        imports: &StylesheetImports,
        mixins: &StylesheetMixins<'_>,
        stack: &mut Vec<String>,
    ) -> Result<TokenStream> {
        match &self.reference {
            MixinReference::Local(name) => {
                let Some(mixin) = mixins.get(name) else {
                    return Err(Error::new_spanned(
                        name,
                        format!("unknown stylesheet mixin `{name}`"),
                    ));
                };

                mixin.expand_with_stack(style, variables, imports, mixins, stack)
            }
            MixinReference::Imported { alias, name } => {
                let module = imports.get(alias)?;
                let name = name.to_string();

                Ok(quote! { (#style).merge(#module.expect_mixin(#name)) })
            }
        }
    }
}

/// Compile-time stylesheet mixin lookup.
#[derive(Default)]
pub(super) struct StylesheetMixins<'a> {
    /// Mixin definitions keyed by source identifier.
    values: HashMap<String, &'a Mixin>,
}

impl<'a> StylesheetMixins<'a> {
    /// Adds one parsed mixin definition.
    ///
    /// # Arguments
    ///
    /// * `mixin` — Parsed mixin definition to store.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if a mixin with the same name was already stored.
    pub(super) fn insert(&mut self, mixin: &'a Mixin) -> Result<()> {
        let name = mixin.name().to_string();

        if self.values.contains_key(&name) {
            return Err(Error::new_spanned(
                mixin.name(),
                format!("duplicate stylesheet mixin `{name}`"),
            ));
        }

        self.values.insert(name, mixin);
        Ok(())
    }

    /// Looks up a mixin by identifier.
    ///
    /// # Arguments
    ///
    /// * `name` — Mixin identifier to look up.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the stored mixin when it exists.
    pub(super) fn get(&self, name: &Ident) -> Option<&'a Mixin> {
        self.values.get(&name.to_string()).copied()
    }
}

/// Returns whether the stream starts with a top-level mixin definition.
///
/// # Arguments
///
/// * `input` — Macro input stream positioned at a top-level stylesheet item.
///
/// # Returns
///
/// A [`bool`] indicating whether `@mixin` is present.
pub(super) fn starts_mixin(input: ParseStream<'_>) -> bool {
    let fork = input.fork();
    fork.parse::<Token![@]>().is_ok() && fork.parse::<kw::mixin>().is_ok()
}
