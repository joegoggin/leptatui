//! Selector model for `stylesheet!` syntax.
//!
//! This module parses type, class, id, focus, active, insert, visual, visited,
//! compound pseudo, nested parent-pseudo, and BEM parent-suffix selectors and
//! lowers them into public `StyleSelector` constructor calls.

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Error, Ident, LitStr, Result, Token,
    parse::{Parse, ParseStream},
};

/// Parsed terminal stylesheet selector.
pub(super) enum Selector {
    /// View type selector such as `Text`.
    Type(Ident),
    /// Class selector such as `.primary`.
    Class(SelectorName),
    /// Id selector such as `#submit`.
    Id(SelectorName),
    /// Pseudo selector such as `:focus`, `:active`, `:insert`, `:visual`, or
    /// `:visited`.
    Pseudo(Ident),
    /// Compound type and pseudo selector such as `Button:focus`, `A:active`, or
    /// `Input:insert`, `TextArea:visual`, or `Link:visited`.
    TypePseudo {
        /// View type part of the compound selector.
        view_type: Ident,
        /// Pseudo-selector part of the compound selector.
        pseudo: Ident,
    },
    /// Nested parent pseudo selector containing the pseudo identifier from
    /// selectors such as `&:focus`, `&:active`, `&:insert`, `&:visual`, or
    /// `&:visited`.
    ParentPseudo(Ident),
    /// Nested BEM suffix concatenated with the nearest parent class selector.
    ParentSuffix(SelectorSuffix),
}

impl Parse for Selector {
    /// Parses a stylesheet selector.
    ///
    /// # Arguments
    ///
    /// * `input` — Macro input stream positioned at a selector.
    ///
    /// # Returns
    ///
    /// A [`Selector`] containing the parsed selector model.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the selector is not a supported type, class,
    /// id, focus, active, insert, visual, visited, type-pseudo, nested
    /// parent-pseudo, or BEM parent-suffix selector.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if input.peek(Token![&]) {
            input.parse::<Token![&]>()?;

            if input.peek(Token![:]) {
                input.parse::<Token![:]>()?;
                return Ok(Self::ParentPseudo(input.parse()?));
            }

            if input.peek(Ident) {
                let suffix: Ident = input.parse()?;
                let value = suffix.to_string();
                if value.starts_with("__") && value.len() > 2 {
                    return Ok(Self::ParentSuffix(SelectorSuffix {
                        value,
                        span: suffix.span(),
                    }));
                }

                return Err(Error::new_spanned(
                    suffix,
                    "stylesheet! parent class suffix must use &__element or &--modifier",
                ));
            }

            if input.peek(Token![-]) {
                let first: Token![-] = input.parse()?;
                if !input.peek(Token![-]) {
                    return Err(Error::new_spanned(
                        first,
                        "stylesheet! parent class suffix must use &__element or &--modifier",
                    ));
                }
                input.parse::<Token![-]>()?;
                let name = SelectorName::parse(input)?;
                return Ok(Self::ParentSuffix(SelectorSuffix {
                    value: format!("--{}", name.value),
                    span: name.span,
                }));
            }

            return Err(input.error(
                "stylesheet! parent selector only supports &:focus, &:active, &:insert, &:visual, &:visited, &__element, or &--modifier in nested rules",
            ));
        }

        if input.peek(Token![.]) {
            input.parse::<Token![.]>()?;
            return Ok(Self::Class(SelectorName::parse(input)?));
        }

        if input.peek(Token![#]) {
            input.parse::<Token![#]>()?;
            return Ok(Self::Id(SelectorName::parse(input)?));
        }

        if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            return Ok(Self::Pseudo(input.parse()?));
        }

        if input.peek(Ident) {
            let view_type = input.parse()?;
            if input.peek(Token![:]) {
                input.parse::<Token![:]>()?;
                let pseudo = input.parse()?;
                return Ok(Self::TypePseudo { view_type, pseudo });
            }

            return Ok(Self::Type(view_type));
        }

        Err(input.error(
            "stylesheet! selector must be a type, .class, #id, :focus, :active, :insert, :visual, :visited, Type:pseudo, nested &:pseudo, or nested BEM parent-suffix selector",
        ))
    }
}

impl Selector {
    /// Expands this selector into a `StyleSelector` expression.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing a public `StyleSelector` constructor call.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if the selector uses an unsupported view type or
    /// pseudo-selector, or if a parent-reference selector is expanded without a
    /// selector path.
    pub(super) fn expand(&self) -> Result<TokenStream> {
        let leptatui = crate::crate_path::leptatui();

        match self {
            Self::Type(view_type) => {
                let view_type = Self::expand_view_type(view_type)?;
                Ok(quote! { #leptatui::StyleSelector::view_type(#view_type) })
            }
            Self::Class(class) => {
                let class = class.literal();
                Ok(quote! { #leptatui::StyleSelector::class(#class) })
            }
            Self::Id(id) => {
                let id = id.literal();
                Ok(quote! { #leptatui::StyleSelector::id(#id) })
            }
            Self::Pseudo(pseudo) => Self::expand_pseudo(pseudo),
            Self::TypePseudo { view_type, pseudo } => {
                let view_type = Self::expand_view_type(view_type)?;
                let pseudo = Self::expand_pseudo(pseudo)?;

                Ok(quote! {
                    #leptatui::StyleSelector::compound(::std::vec![
                        #leptatui::StyleSelector::view_type(#view_type),
                        #pseudo,
                    ])
                })
            }
            Self::ParentPseudo(_) | Self::ParentSuffix(_) => Err(Error::new_spanned(
                self.span_tokens(),
                "stylesheet! parent-reference selector can only appear inside a nested rule",
            )),
        }
    }

    /// Expands a selector path into a single runtime selector expression.
    ///
    /// # Arguments
    ///
    /// * `path` — Ordered selector path from outermost rule to current rule.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing a public `StyleSelector` expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if parent-reference selectors appear without a
    /// parent selector or any selector segment cannot be expanded.
    pub(super) fn expand_path(path: &[&Selector]) -> Result<TokenStream> {
        let leptatui = crate::crate_path::leptatui();
        let mut segments = Vec::<SelectorSegment>::new();

        for selector in path {
            match selector {
                Self::ParentPseudo(pseudo) => {
                    let pseudo = Self::expand_pseudo(pseudo)?;
                    let Some(segment) = segments.last_mut() else {
                        return Err(Error::new_spanned(
                            selector.span_tokens(),
                            "stylesheet! parent pseudo-selector requires a parent selector",
                        ));
                    };

                    segment.compounds.push(pseudo);
                }
                Self::ParentSuffix(suffix) => {
                    let Some(segment) = segments.last_mut() else {
                        return Err(Error::new_spanned(
                            selector.span_tokens(),
                            "stylesheet! parent class suffix requires a parent selector",
                        ));
                    };

                    let Some(class) = segment.class.as_mut() else {
                        return Err(Error::new_spanned(
                            selector.span_tokens(),
                            "stylesheet! parent class suffix requires a class selector",
                        ));
                    };
                    if !segment.compounds.is_empty() {
                        return Err(Error::new_spanned(
                            selector.span_tokens(),
                            "stylesheet! parent class suffix cannot follow a pseudo-selector",
                        ));
                    }

                    class.value.push_str(&suffix.value);
                }
                Self::Class(class) => segments.push(SelectorSegment::class(class)),
                _ => segments.push(SelectorSegment::expanded(selector.expand()?)),
            }
        }

        let Some(target) = segments.pop() else {
            return Err(Error::new(
                proc_macro2::Span::call_site(),
                "stylesheet! rule requires a selector",
            ));
        };

        let target = target.expand(&leptatui);
        if segments.is_empty() {
            return Ok(target);
        }

        let ancestors = segments
            .into_iter()
            .map(|segment| segment.expand(&leptatui));

        Ok(quote! {
            #leptatui::StyleSelector::descendant(
                ::std::vec![#(#ancestors),*],
                #target,
            )
        })
    }

    /// Expands an open semantic view type selector identifier.
    ///
    /// # Arguments
    ///
    /// * `view_type` — Parsed view type identifier to lower.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing an open `ViewType` name.
    fn expand_view_type(view_type: &Ident) -> Result<TokenStream> {
        let leptatui = crate::crate_path::leptatui();
        Ok(quote! { #leptatui::ViewType::new(::core::stringify!(#view_type)) })
    }

    /// Expands a supported pseudo-selector identifier.
    ///
    /// # Arguments
    ///
    /// * `pseudo` — Parsed pseudo-selector identifier to lower.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing a public `StyleSelector` expression.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] if `pseudo` is not supported.
    fn expand_pseudo(pseudo: &Ident) -> Result<TokenStream> {
        let leptatui = crate::crate_path::leptatui();

        match pseudo.to_string().as_str() {
            "focus" => Ok(quote! { #leptatui::StyleSelector::focus() }),
            "active" => Ok(quote! { #leptatui::StyleSelector::active() }),
            "insert" => Ok(quote! { #leptatui::StyleSelector::insert() }),
            "visual" => Ok(quote! { #leptatui::StyleSelector::visual() }),
            "visited" => Ok(quote! { #leptatui::StyleSelector::visited() }),
            _ => Err(Error::new_spanned(
                pseudo,
                "unsupported stylesheet pseudo-selector; expected :focus, :active, :insert, :visual, or :visited",
            )),
        }
    }

    /// Returns tokens that identify this selector in diagnostics.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the selector span source.
    fn span_tokens(&self) -> TokenStream {
        match self {
            Self::Type(ident) | Self::Pseudo(ident) | Self::ParentPseudo(ident) => {
                ident.to_token_stream()
            }
            Self::Class(name) | Self::Id(name) => name.to_token_stream(),
            Self::TypePseudo { view_type, pseudo } => quote! { #view_type : #pseudo },
            Self::ParentSuffix(suffix) => suffix.to_token_stream(),
        }
    }
}

/// Parsed BEM suffix from a nested parent-reference selector.
pub(super) struct SelectorSuffix {
    /// Suffix text, including the leading `__` or `--` delimiter.
    value: String,
    /// Source span used for generated literals and diagnostics.
    span: Span,
}

impl ToTokens for SelectorSuffix {
    /// Appends this selector suffix as a string literal token.
    ///
    /// # Arguments
    ///
    /// * `tokens` — Token stream receiving the suffix literal.
    fn to_tokens(&self, tokens: &mut TokenStream) {
        LitStr::new(&self.value, self.span).to_tokens(tokens);
    }
}

/// Parsed class or id selector name.
pub(super) struct SelectorName {
    value: String,
    span: Span,
}

/// One normalized descendant-selector segment during macro expansion.
struct SelectorSegment {
    /// Class name when the segment supports BEM suffix concatenation.
    class: Option<SelectorName>,
    /// Expanded non-class base selector.
    expanded: Option<TokenStream>,
    /// Pseudo-selectors compounded with the base selector.
    compounds: Vec<TokenStream>,
}

impl SelectorSegment {
    /// Creates a segment backed by a class selector.
    ///
    /// # Arguments
    ///
    /// * `class` — Parsed class selector name.
    ///
    /// # Returns
    ///
    /// A [`SelectorSegment`] that accepts nested BEM suffixes.
    fn class(class: &SelectorName) -> Self {
        Self {
            class: Some(SelectorName {
                value: class.value.clone(),
                span: class.span,
            }),
            expanded: None,
            compounds: Vec::new(),
        }
    }

    /// Creates a segment backed by an already expanded selector.
    ///
    /// # Arguments
    ///
    /// * `expanded` — Runtime selector constructor expression.
    ///
    /// # Returns
    ///
    /// A [`SelectorSegment`] that does not accept BEM suffixes.
    fn expanded(expanded: TokenStream) -> Self {
        Self {
            class: None,
            expanded: Some(expanded),
            compounds: Vec::new(),
        }
    }

    /// Expands the normalized segment into a runtime selector expression.
    ///
    /// # Arguments
    ///
    /// * `leptatui` — Token path to the Leptatui crate used in generated code.
    ///
    /// # Returns
    ///
    /// A [`TokenStream`] containing the class or non-class base and compounds.
    fn expand(self, leptatui: &TokenStream) -> TokenStream {
        let base = self.class.map_or_else(
            || self.expanded.expect("selector segment requires a base"),
            |class| {
                let class = class.literal();
                quote! { #leptatui::StyleSelector::class(#class) }
            },
        );
        let mut selectors = vec![base];
        selectors.extend(self.compounds);
        expand_selector_segment(leptatui, selectors)
    }
}

impl SelectorName {
    /// Parses an identifier name with optional dash-separated identifier segments.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let first: Ident = input.parse()?;
        let span = first.span();
        let mut value = first.to_string();

        while input.peek(Token![-]) {
            input.parse::<Token![-]>()?;
            let segment: Ident = input.parse()?;
            value.push('-');
            value.push_str(&segment.to_string());
        }

        Ok(Self { value, span })
    }

    /// Returns this selector name as a string literal for generated code.
    fn literal(&self) -> LitStr {
        LitStr::new(&self.value, self.span)
    }
}

impl ToTokens for SelectorName {
    /// Appends this selector name as a string literal token.
    ///
    /// # Arguments
    ///
    /// * `tokens` — Token stream receiving the selector literal.
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.literal().to_tokens(tokens);
    }
}

/// Expands one selector path segment into a runtime selector expression.
///
/// # Arguments
///
/// * `leptatui` — Token path to the Leptatui crate used in generated code.
/// * `selectors` — Runtime selector expressions in the same path segment.
///
/// # Returns
///
/// A [`TokenStream`] containing either a single selector expression or a
/// compound selector expression.
fn expand_selector_segment(leptatui: &TokenStream, selectors: Vec<TokenStream>) -> TokenStream {
    if selectors.len() == 1 {
        let mut selectors = selectors.into_iter();
        return selectors.next().expect("checked selector segment length");
    }

    quote! { #leptatui::StyleSelector::compound(::std::vec![#(#selectors),*]) }
}
