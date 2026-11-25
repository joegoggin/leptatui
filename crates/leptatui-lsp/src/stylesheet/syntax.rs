//! Token ranges and balanced groups shared by stylesheet editor features.

use std::ops::Range;

/// One nontrivia Rust token, with source spelling and byte offsets.
#[derive(Clone)]
pub(crate) struct Token<'a> {
    /// Original token spelling.
    pub text: &'a str,
    /// Original byte range.
    pub span: Range<usize>,
    /// Whether the token is an identifier.
    pub ident: bool,
    /// Whether the token is a literal.
    pub literal: bool,
}

/// An invocation whose body uses stylesheet or markup syntax.
#[derive(Clone)]
pub(crate) struct Macro {
    /// Index of the opening delimiter.
    pub open: usize,
    /// Index of the closing delimiter, or token count during recovery.
    pub end: usize,
    /// Byte range of the invocation contents.
    pub span: Range<usize>,
    /// Whether this invocation contains terminal markup.
    pub view: bool,
}

/// Tokenized Rust source with recoverable delimiter pairs and macro bodies.
pub(crate) struct Syntax<'a> {
    /// Source with unsaved edits applied.
    pub text: &'a str,
    /// Nontrivia tokens.
    pub tokens: Vec<Token<'a>>,
    /// Matching closing delimiter for each opening token.
    pub pairs: Vec<Option<usize>>,
    /// Recognized stylesheet and markup invocations.
    pub macros: Vec<Macro>,
    /// Comments and strings where stylesheet completion must not run.
    excluded: Vec<Range<usize>>,
}

impl<'a> Syntax<'a> {
    /// Tokenizes source without requiring a valid Rust or stylesheet parse.
    ///
    /// # Arguments
    ///
    /// * `text` — Source document with unsaved edits applied.
    ///
    /// # Returns
    ///
    /// A recovering token and macro index for the source.
    pub fn new(text: &'a str) -> Self {
        use rustc_lexer::TokenKind;
        let mut offset = 0;
        let mut tokens = Vec::new();
        let mut excluded = Vec::new();
        for token in rustc_lexer::tokenize(text) {
            let span = offset..offset + token.len;
            offset = span.end;
            let comment = matches!(
                token.kind,
                TokenKind::LineComment | TokenKind::BlockComment { .. }
            );
            let literal = matches!(token.kind, TokenKind::Literal { .. });
            if comment || literal {
                let inclusive = matches!(
                    token.kind,
                    TokenKind::LineComment | TokenKind::BlockComment { terminated: false }
                ) || matches!(
                    token.kind,
                    TokenKind::Literal {
                        kind: rustc_lexer::LiteralKind::Str { terminated: false }
                            | rustc_lexer::LiteralKind::ByteStr { terminated: false }
                            | rustc_lexer::LiteralKind::RawStr {
                                terminated: false,
                                ..
                            }
                            | rustc_lexer::LiteralKind::RawByteStr {
                                terminated: false,
                                ..
                            },
                        ..
                    }
                );
                excluded.push(span.start..span.end + usize::from(inclusive));
            }
            if !comment && token.kind != TokenKind::Whitespace {
                tokens.push(Token {
                    text: &text[span.clone()],
                    span,
                    ident: matches!(token.kind, TokenKind::Ident | TokenKind::RawIdent),
                    literal,
                });
            }
        }
        let mut pairs = vec![None; tokens.len()];
        let mut stack = Vec::new();
        for (i, t) in tokens.iter().enumerate() {
            match t.text {
                "{" | "(" | "[" => stack.push(i),
                "}" | ")" | "]" => {
                    let expected = match t.text {
                        "}" => "{",
                        ")" => "(",
                        _ => "[",
                    };
                    if let Some(n) = stack.iter().rposition(|&j| tokens[j].text == expected) {
                        for open in stack.drain(n..) {
                            pairs[open] = Some(i);
                        }
                    }
                }
                _ => {}
            }
        }
        let mut names = vec![("stylesheet", false), ("view", true)];
        for w in tokens.windows(3) {
            if matches!(w[0].text, "stylesheet" | "view") && w[1].text == "as" && w[2].ident {
                names.push((w[2].text, w[0].text == "view"));
            }
        }
        let mut macros = Vec::new();
        for i in 2..tokens.len() {
            if tokens[i - 1].text != "!" || !matches!(tokens[i].text, "{" | "(" | "[") {
                continue;
            }
            if let Some((_, view)) = names.iter().find(|(name, _)| *name == tokens[i - 2].text) {
                let end = pairs[i].unwrap_or(tokens.len());
                macros.push(Macro {
                    open: i,
                    end,
                    span: tokens[i].span.end..tokens.get(end).map_or(text.len(), |t| t.span.start),
                    view: *view,
                });
            }
        }
        Self {
            text,
            tokens,
            pairs,
            macros,
            excluded,
        }
    }

    /// Finds the innermost stylesheet body at an editing position.
    ///
    /// # Arguments
    ///
    /// * `cursor` — Request byte offset at a UTF-8 boundary.
    ///
    /// # Returns
    ///
    /// An optional innermost stylesheet invocation.
    pub fn stylesheet(&self, cursor: usize) -> Option<&Macro> {
        self.macros
            .iter()
            .filter(|m| !m.view && m.span.start <= cursor && cursor <= m.span.end)
            .max_by_key(|m| m.open)
    }

    /// Reports whether the position lies inside a comment or literal.
    ///
    /// # Arguments
    ///
    /// * `cursor` — Request byte offset at a UTF-8 boundary.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether syntax completion must be suppressed.
    pub fn excluded(&self, cursor: usize) -> bool {
        self.excluded
            .iter()
            .any(|r| r.start < cursor && cursor < r.end)
    }

    /// Returns a group boundary or the containing block's recovery boundary.
    ///
    /// # Arguments
    ///
    /// * `index` — Source token index.
    /// * `limit` — Containing block boundary used during recovery.
    ///
    /// # Returns
    ///
    /// A matching or recovered closing delimiter index.
    pub fn end(&self, index: usize, limit: usize) -> usize {
        self.pairs
            .get(index)
            .copied()
            .flatten()
            .unwrap_or(limit)
            .min(limit)
    }

    /// Returns a replaceable identifier, including dash-separated selector segments.
    ///
    /// # Arguments
    ///
    /// * `cursor` — Request byte offset at a UTF-8 boundary.
    ///
    /// # Returns
    ///
    /// A byte range covering the identifier or selector word.
    pub fn word(&self, cursor: usize) -> Range<usize> {
        if let Some(token) = self.tokens.iter().find(|t| {
            t.ident && t.text.starts_with("r#") && t.span.start <= cursor && cursor <= t.span.end
        }) {
            return token.span.clone();
        }
        let mut start = cursor;
        let mut end = cursor;
        let word = |c: char| c.is_alphanumeric() || c == '_' || c == '-';
        while start > 0 && self.text[..start].chars().next_back().is_some_and(word) {
            start -= self.text[..start].chars().next_back().unwrap().len_utf8();
        }
        while end < self.text.len() && self.text[end..].chars().next().is_some_and(word) {
            end += self.text[end..].chars().next().unwrap().len_utf8();
        }
        start..end
    }

    /// Returns token spelling, or an empty sentinel beyond the source.
    ///
    /// # Arguments
    ///
    /// * `index` — Source token index.
    ///
    /// # Returns
    ///
    /// A token spelling, or an empty sentinel beyond the source.
    pub fn at(&self, index: usize) -> &str {
        self.tokens.get(index).map_or("", |t| t.text)
    }
}

/// Grammar role of a recoverable statement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// Variable declaration.
    Variable,
    /// Style module import.
    Use,
    /// Mixin definition.
    Mixin,
    /// Mixin inclusion.
    Include,
    /// Viewport query.
    Media,
    /// Style property declaration.
    Property,
    /// Terminal selector.
    Rule,
}

/// Statement header and optional nested stylesheet body.
pub(crate) struct Statement {
    /// Grammar role.
    pub kind: Kind,
    /// Header tokens, excluding a body or trailing semicolon.
    pub head: Range<usize>,
    /// Complete authored range including terminator or body.
    pub span: Range<usize>,
    /// Nested statements for selectors, media, and mixins.
    pub body: Option<Block>,
    /// Whether a semicolon-terminated statement has its terminator.
    pub terminated: bool,
}

/// Stylesheet block whose Rust value groups are kept opaque.
pub(crate) struct Block {
    /// Inner source byte range.
    pub span: Range<usize>,
    /// Whether declarations are accepted here.
    pub declarations: bool,
    /// Whether nested rules are accepted here.
    pub rules: bool,
    /// Statements parsed despite incomplete headers or values.
    pub statements: Vec<Statement>,
}

impl Syntax<'_> {
    /// Recovers an invocation's structural stylesheet tree.
    ///
    /// # Arguments
    ///
    /// * `m` — Containing macro invocation.
    ///
    /// # Returns
    ///
    /// A recovering stylesheet block tree.
    pub fn parse(&self, m: &Macro) -> Block {
        self.block(m.open, m.end, false, true)
    }

    /// Parses statements while retaining Rust expression groups as opaque tokens.
    ///
    /// # Arguments
    ///
    /// * `open` — Opening delimiter token index.
    /// * `end` — Closing delimiter or recovery boundary token index.
    /// * `declarations` — Whether the block permits style declarations.
    /// * `rules` — Whether the block permits nested selectors.
    ///
    /// # Returns
    ///
    /// A recovering block with its structural statements.
    fn block(&self, open: usize, end: usize, declarations: bool, rules: bool) -> Block {
        let mut statements = Vec::new();
        let mut i = open + 1;
        while i < end {
            if matches!(self.at(i), ";" | ",") {
                i += 1;
                continue;
            }
            let start = i;
            let kind = if self.at(i) == "$" {
                Kind::Variable
            } else if self.at(i) == "@" {
                match self.at(i + 1) {
                    "use" => Kind::Use,
                    "mixin" => Kind::Mixin,
                    "include" => Kind::Include,
                    "media" => Kind::Media,
                    _ => Kind::Rule,
                }
            } else if self.tokens[i].ident
                && self.at(i + 1) == ":"
                && (super::properties::PROPERTIES
                    .iter()
                    .any(|p| p.0 == self.at(i))
                    || !(self.tokens[i]
                        .text
                        .chars()
                        .next()
                        .is_some_and(char::is_uppercase)
                        || self.tokens.get(i + 2).is_some_and(|t| t.ident)
                            && self.at(i + 3) == "{"))
            {
                Kind::Property
            } else {
                Kind::Rule
            };
            let structural = matches!(kind, Kind::Mixin | Kind::Media | Kind::Rule);
            let mut body = None;
            let mut head_end = end;
            let mut terminated = false;
            while i < end {
                if self.at(i) == ";" {
                    head_end = i;
                    i += 1;
                    terminated = true;
                    break;
                }
                if structural && self.at(i) == "{" {
                    head_end = i;
                    let close = self.end(i, end);
                    let decl = kind != Kind::Media || declarations;
                    body = Some(self.block(i, close, decl, kind != Kind::Mixin));
                    i = if close < end { close + 1 } else { close };
                    break;
                }
                if matches!(self.at(i), "{" | "(" | "[") {
                    i = self.end(i, end);
                    if i >= end {
                        break;
                    }
                }
                // A following declaration is a useful recovery boundary for a missing semicolon.
                if !structural
                    && i > start + 2
                    && (self.at(i) == "@"
                        || self.tokens[i].ident && self.at(i + 1) == ":" && self.at(i + 2) != ":")
                {
                    head_end = i;
                    break;
                }
                i += 1;
            }
            if i == start {
                i += 1;
            }
            statements.push(Statement {
                kind,
                head: start..head_end,
                span: self.tokens[start].span.start
                    ..self
                        .tokens
                        .get(i.saturating_sub(1))
                        .map_or(self.text.len(), |t| t.span.end),
                body,
                terminated,
            });
        }
        Block {
            span: self.tokens[open].span.end
                ..self
                    .tokens
                    .get(end)
                    .map_or(self.text.len(), |t| t.span.start),
            declarations,
            rules,
            statements,
        }
    }
}

impl Block {
    /// Finds the deepest stylesheet block containing a cursor.
    ///
    /// # Arguments
    ///
    /// * `cursor` — Request byte offset at a UTF-8 boundary.
    ///
    /// # Returns
    ///
    /// A containing block and optional current statement.
    pub fn context(&self, cursor: usize) -> (&Block, Option<&Statement>) {
        for (index, statement) in self.statements.iter().enumerate() {
            if let Some(body) = &statement.body
                && body.span.start <= cursor
                && cursor <= body.span.end
            {
                return body.context(cursor);
            }
            let end = if !statement.terminated && statement.body.is_none() {
                self.statements
                    .get(index + 1)
                    .map_or(self.span.end, |s| s.span.start)
            } else {
                statement.span.end
            };
            if statement.span.start <= cursor && cursor <= end {
                return (self, Some(statement));
            }
        }
        (self, None)
    }

    /// Visits statements in source order, including nested stylesheet blocks.
    ///
    /// # Arguments
    ///
    /// * `output` — Destination for collected results.
    pub fn walk<'a>(&'a self, output: &mut Vec<&'a Statement>) {
        for statement in &self.statements {
            output.push(statement);
            if let Some(body) = &statement.body {
                body.walk(output);
            }
        }
    }
}
