//! Structural stylesheet printing with source-preserving comments and literals.

use super::*;

/// Returns comment tokens with their source byte ranges.
///
/// # Arguments
///
/// * `text` — Source fragment.
///
/// # Returns
///
/// A list of authored comments in order.
pub(super) fn comments(text: &str) -> Vec<(Range<usize>, &str)> {
    let mut offset = 0;
    rustc_lexer::tokenize(text)
        .filter_map(|token| {
            let span = offset..offset + token.len;
            offset = span.end;
            matches!(
                token.kind,
                rustc_lexer::TokenKind::LineComment | rustc_lexer::TokenKind::BlockComment { .. }
            )
            .then(|| (span.clone(), &text[span]))
        })
        .collect()
}

/// Shifts code indentation without changing multiline literals or comment contents.
///
/// # Arguments
///
/// * `text` — Text with relative continuation indentation.
/// * `prefix` — Indentation prepended to code lines.
/// * `remove` — Existing indentation columns removed from continuation lines.
/// * `tab_size` — Columns represented by a hard tab.
///
/// # Returns
///
/// A source fragment with adjusted code indentation.
pub(super) fn shift(text: &str, prefix: &str, remove: usize, tab_size: usize) -> String {
    let mut offset = 0;
    let protected: Vec<_> = rustc_lexer::tokenize(text)
        .filter_map(|token| {
            let span = offset..offset + token.len;
            offset = span.end;
            matches!(
                token.kind,
                rustc_lexer::TokenKind::Literal { .. }
                    | rustc_lexer::TokenKind::BlockComment { .. }
            )
            .then_some(span)
        })
        .collect();
    let mut output = String::new();
    offset = 0;
    for (index, line) in text.split_inclusive('\n').enumerate() {
        if protected.iter().any(|r| r.start < offset && offset < r.end) {
            output.push_str(line);
        } else {
            let mut columns = 0;
            let bytes = if index == 0 {
                0
            } else {
                line.char_indices()
                    .take_while(|(_, c)| {
                        if columns >= remove {
                            return false;
                        }
                        match c {
                            ' ' => columns += 1,
                            '\t' => columns += tab_size,
                            _ => return false,
                        }
                        true
                    })
                    .map(|(i, c)| i + c.len_utf8())
                    .last()
                    .unwrap_or(0)
            };
            output.push_str(prefix);
            output.push_str(&line[bytes..]);
        }
        offset += line.len();
    }
    output
}

/// Normalizes a selector or directive header without changing comment tokens.
///
/// # Arguments
///
/// * `syntax` — Source tokens.
/// * `s` — Statement header.
///
/// # Returns
///
/// A header with stylesheet punctuation spacing.
fn header(syntax: &Syntax<'_>, s: &Statement) -> String {
    let tokens = &syntax.tokens[s.head.clone()];
    let raw = &syntax.text[tokens[0].span.start..tokens.last().unwrap().span.end];
    if !comments(raw).is_empty() {
        return raw.into();
    }
    let mut output = String::new();
    for (i, token) in tokens.iter().enumerate() {
        let previous = i.checked_sub(1).map_or("", |n| tokens[n].text);
        if (i == 2
            && matches!(
                s.kind,
                Kind::Use | Kind::Include | Kind::Mixin | Kind::Media
            ))
            || (s.kind == Kind::Media
                && (token.text == "and" || previous == "and" || previous == ":"))
            || (s.kind == Kind::Use && (token.text == "as" || previous == "as"))
        {
            output.push(' ');
        }
        output.push_str(token.text);
    }
    output
}

/// Prints a semicolon-terminated declaration or directive.
///
/// # Arguments
///
/// * `syntax` — Source tokens and values.
/// * `s` — Leaf statement.
/// * `options` — Layout preferences.
/// * `expressions` — Rustfmt results indexed by statement start.
///
/// # Returns
///
/// A statement with relative continuation indentation.
fn simple(
    syntax: &Syntax<'_>,
    s: &Statement,
    options: &Options,
    expressions: &HashMap<usize, String>,
) -> String {
    if let Some((range, suffix)) = value(syntax, s) {
        let raw = syntax.text[range].trim();
        let line = syntax.text[..s.span.start]
            .rsplit('\n')
            .next()
            .unwrap_or("");
        let remove = line
            .chars()
            .take_while(|c| matches!(c, ' ' | '\t'))
            .map(|c| if c == '\t' { options.tab_size } else { 1 })
            .sum();
        let mut expression = expressions
            .get(&s.span.start)
            .cloned()
            .unwrap_or_else(|| shift(raw, "", remove, options.tab_size));
        if stylesheet_reference(raw) && comments(raw).is_empty() {
            expression = Syntax::new(raw).tokens.iter().map(|t| t.text).collect();
        }
        if !suffix.is_empty()
            && rustc_lexer::tokenize(&expression)
                .filter(|t| t.kind != rustc_lexer::TokenKind::Whitespace)
                .last()
                .is_some_and(|t| t.kind == rustc_lexer::TokenKind::LineComment)
        {
            expression.push('\n');
        }
        let colon = s.head.start + if s.kind == Kind::Variable { 2 } else { 1 };
        let name: String = syntax.tokens[s.head.start..colon]
            .iter()
            .map(|t| t.text)
            .collect();
        // Comments between the property and its colon belong to the declaration name.
        let name_raw = &syntax.text[s.span.start..syntax.tokens[colon].span.start];
        let name = if comments(name_raw).is_empty() {
            name
        } else {
            name_raw.trim().into()
        };
        terminate(format!("{name}: {expression}{suffix}"))
    } else {
        let end = syntax.tokens[s.head.end].span.start;
        let trailing = &syntax.text[syntax.tokens[s.head.end - 1].span.end..end];
        terminate(format!("{}{}", header(syntax, s), trailing.trim_end()))
    }
}

/// Appends a terminator without letting a trailing line comment consume it.
///
/// # Arguments
///
/// * `text` — Statement text before its terminator.
///
/// # Returns
///
/// A semicolon-terminated statement.
fn terminate(mut text: String) -> String {
    if rustc_lexer::tokenize(&text)
        .filter(|t| t.kind != rustc_lexer::TokenKind::Whitespace)
        .last()
        .is_some_and(|t| t.kind == rustc_lexer::TokenKind::LineComment)
    {
        text.push('\n');
    }
    text.push(';');
    text
}

/// Inserts one blank line when a structural group needs separation.
///
/// # Arguments
///
/// * `output` — Accumulated block text.
fn blank(output: &mut String) {
    if !output.is_empty() && !output.ends_with("\n\n") {
        output.push('\n');
    }
}

/// Emits gap comments, retaining same-line trailing comment attachments.
///
/// # Arguments
///
/// * `output` — Accumulated block text.
/// * `gap` — Trivia between statements.
/// * `indent` — Current statement indentation.
/// * `separate` — Whether a new structural group needs a blank line.
/// * `options` — Indentation preferences.
fn gap(output: &mut String, gap: &str, indent: &str, separate: bool, options: &Options) {
    let mut end = 0;
    let mut standalone = false;
    for (span, comment) in comments(gap) {
        let attach = !output.is_empty() && !gap[end..span.start].contains('\n');
        if attach {
            if output.ends_with('\n') {
                output.pop();
            }
            output.push(' ');
            output.push_str(comment);
            output.push('\n');
        } else {
            if separate && !standalone {
                blank(output);
            }
            output.push_str(&shift(comment, indent, 0, options.tab_size));
            output.push('\n');
            standalone = true;
        }
        end = span.end;
    }
    if separate && !standalone {
        blank(output);
    }
}

/// Prints a block body with normalized declaration and rule grouping.
///
/// # Arguments
///
/// * `syntax` — Current source index.
/// * `body` — Complete block body.
/// * `base` — Indentation of the containing opening delimiter.
/// * `options` — Layout preferences.
/// * `expressions` — Rustfmt values indexed by declaration start.
///
/// # Returns
///
/// A block body ending in a newline when nonempty.
pub(super) fn block(
    syntax: &Syntax<'_>,
    body: &Block,
    base: &str,
    options: &Options,
    expressions: &HashMap<usize, String>,
) -> String {
    let indent = format!("{base}{}", options.indent());
    let mut output = String::new();
    let mut end = body.span.start;
    let mut previous: Option<&Statement> = None;
    for s in &body.statements {
        let separate = previous.is_some_and(|p| {
            p.body.is_some()
                || s.body.is_some()
                || p.kind != s.kind && matches!(p.kind, Kind::Use | Kind::Variable)
        });
        gap(
            &mut output,
            &syntax.text[end..s.span.start],
            &indent,
            separate,
            options,
        );
        if let Some(child) = &s.body {
            let head = header(syntax, s);
            let single = child
                .statements
                .first()
                .filter(|_| child.statements.len() == 1)
                .filter(|c| c.body.is_none())
                .map(|c| simple(syntax, c, options, expressions));
            let inline = single
                .as_ref()
                .map(|single| format!("{head} {{ {single} }}"));
            let width = indent
                .chars()
                .map(|c| if c == '\t' { options.tab_size } else { 1 })
                .sum::<usize>();
            let compact = s.kind == Kind::Rule
                && comments(&syntax.text[s.span.clone()]).is_empty()
                && inline.as_ref().is_some_and(|text| {
                    !text.contains('\n') && width + text.chars().count() <= options.width
                });
            if compact {
                output.push_str(&indent);
                output.push_str(inline.as_ref().unwrap());
                output.push('\n');
            } else if child.statements.is_empty()
                && comments(&syntax.text[child.span.clone()]).is_empty()
                && s.kind == Kind::Rule
            {
                output.push_str(&format!("{indent}{head} {{}}\n"));
            } else {
                output.push_str(&shift(&head, &indent, 0, options.tab_size));
                output.push_str(" {\n");
                // Retain comments between the final header token and the opening brace.
                let open = syntax.tokens[s.head.end].span.start;
                let header_end = syntax.tokens[s.head.end - 1].span.end;
                let extra = &syntax.text[header_end..open];
                gap(
                    &mut output,
                    &format!("\n{extra}"),
                    &format!("{indent}{}", options.indent()),
                    false,
                    options,
                );
                output.push_str(&block(syntax, child, &indent, options, expressions));
                output.push_str(&format!("{indent}}}\n"));
            }
        } else {
            output.push_str(&shift(
                &simple(syntax, s, options, expressions),
                &indent,
                0,
                options.tab_size,
            ));
            output.push('\n');
        }
        end = s.span.end;
        previous = Some(s);
    }
    gap(
        &mut output,
        &syntax.text[end..body.span.end],
        &indent,
        false,
        options,
    );
    output
}
