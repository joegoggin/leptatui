//! Expected-value templates and recovering declaration value contexts.
//!
//! # Modules
//!
//! - [`catalog`] — Enum choices, constructors, and wrapped layout expressions.

mod catalog;
use catalog::templates;

use super::{
    properties, range,
    syntax::{Kind, Syntax},
};
use serde_json::{Value, json};
use std::ops::Range;

/// A property value position where expected-value templates are useful.
struct Context {
    /// Shared compiler value category.
    category: &'static str,
    /// Property spelling used for documentation.
    property: String,
    /// Authored path prefix through the cursor.
    prefix: String,
    /// Full path range, including any suffix after the cursor.
    span: Range<usize>,
}

impl Context {
    /// Finds an empty value, simple path, or subsequent flag operand.
    ///
    /// # Arguments
    ///
    /// * `text` — Current source overlay.
    /// * `cursor` — Cursor byte offset.
    ///
    /// # Returns
    ///
    /// An optional expected-value context; compound Rust expressions remain upstream.
    fn find(text: &str, cursor: usize) -> Option<Self> {
        let syntax = Syntax::new(text);
        let m = syntax.stylesheet(cursor)?;
        if syntax.excluded(cursor) {
            return None;
        }
        let tree = syntax.parse(m);
        let (block, statement) = tree.context(cursor);
        let statement = statement?;
        if !block.declarations || statement.kind != Kind::Property {
            return None;
        }
        let head = statement.head.clone();
        let property = syntax.at(head.start);
        let category = properties::PROPERTIES.iter().find(|p| p.0 == property)?.1;
        let colon = syntax.tokens.get(head.start + 1)?;
        if colon.span.end > cursor {
            return None;
        }
        let mut start = colon.span.end;
        let mut operand = Vec::new();
        for token in &syntax.tokens[head.start + 2..head.end] {
            if token.span.start >= cursor {
                break;
            }
            if token.text == "|" && matches!(category, "Modifier" | "Borders") {
                // Only simple flag paths may precede the next operand.
                if operand.is_empty()
                    || operand
                        .iter()
                        .any(|t: &&super::syntax::Token<'_>| !t.ident && t.text != ":")
                {
                    return None;
                }
                start = token.span.end;
                operand.clear();
            } else {
                operand.push(token);
            }
        }
        if operand.iter().any(|t| !t.ident && t.text != ":") {
            return None;
        }
        let first = operand.first().map_or(cursor, |t| t.span.start);
        let prefix = text[first..cursor].trim().to_owned();
        // Whitespace between path tokens is permitted, but separate identifiers are not.
        let compact: String = prefix.chars().filter(|c| !c.is_whitespace()).collect();
        if !compact.is_empty()
            && syn::parse_str::<syn::Path>(&format!("{}__completion", compact)).is_err()
        {
            return None;
        }
        let word = syntax.word(cursor);
        let span = if operand.is_empty() {
            cursor..word.end
        } else {
            first.max(start)..word.end
        };
        Some(Self {
            category,
            property: property.into(),
            prefix: compact,
            span,
        })
    }
}

/// Expands snippet defaults for clients without snippet support.
///
/// # Arguments
///
/// * `expression` — Catalog expression with placeholders.
/// * `snippets` — Whether placeholders should be retained.
///
/// # Returns
///
/// A Rust expression or snippet using the catalog's unqualified type names.
fn render(expression: &str, snippets: bool) -> String {
    if snippets {
        return expression.to_owned();
    }
    let mut output = String::new();
    let mut rest = expression;
    while let Some((before, after)) = rest.split_once("${") {
        output.push_str(before);
        let (placeholder, tail) = after.split_once('}').expect("catalog placeholder closes");
        let (_, default) = placeholder
            .split_once(':')
            .expect("catalog placeholder has a default");
        output.push_str(default);
        rest = tail;
    }
    output.push_str(rest);
    output
}

/// Builds expected-value completions without replacing upstream Rust results.
///
/// # Arguments
///
/// * `text` — Current source overlay.
/// * `cursor` — Completion byte offset.
/// * `snippets` — Client snippet capability.
///
/// # Returns
///
/// A list of matching expected-value items.
pub(crate) fn complete(text: &str, cursor: usize, snippets: bool) -> Vec<Value> {
    let Some(context) = Context::find(text, cursor) else {
        return Vec::new();
    };
    let prefix = context.prefix.to_lowercase();
    let qualifier = context
        .prefix
        .rsplit_once("::")
        .map(|(path, _)| format!("{path}::"));
    let mut output = Vec::new();
    for template in templates(context.category) {
        if context.property == "aspect_ratio" && template.label == "0.0" {
            continue;
        }
        let member = template.label.split("::").nth(1).unwrap_or(&template.label);
        let matches = template.label.to_lowercase().starts_with(&prefix)
            || member.to_lowercase().starts_with(&prefix);
        let qualified = qualifier.as_ref().and_then(|q| {
            let ty = q.trim_end_matches("::").rsplit("::").next()?;
            let suffix = template.expression.strip_prefix(&format!("{ty}::"))?;
            let partial = context.prefix.rsplit("::").next().unwrap_or("");
            suffix
                .to_lowercase()
                .starts_with(&partial.to_lowercase())
                .then_some((q, suffix))
        });
        if !matches && qualified.is_none() {
            continue;
        }
        let expression = if let Some((q, suffix)) = qualified {
            // Preserve the authored type path and leave argument types unqualified.
            format!("{q}{}", render(suffix, snippets))
        } else {
            render(&template.expression, snippets)
        };
        let filter = if let Some((q, suffix)) = qualified {
            format!("{q}{}", suffix.split('(').next().unwrap_or(suffix))
        } else if member.to_lowercase().starts_with(&prefix) && !prefix.is_empty() {
            member.to_owned()
        } else {
            template.label.clone()
        };
        output.push(json!({
            "label":template.label, "kind":if template.expression.contains("${") {15} else {20},
            "detail":format!("{} — {}", context.property, super::property_type(context.category)),
            "documentation":{"kind":"markdown","value":format!("{}\n\n```rust\n{}\n```", template.description, render(&template.expression, false))},
            "filterText":filter,"sortText":format!("0_{}", template.label),
            "textEdit":{"range":range(text,context.span.clone()),"newText":expression},
            "insertTextFormat":if snippets {2} else {1}, "data":{"leptatuiStylesheet":true}
        }));
    }
    output
}

#[cfg(test)]
mod tests;

/// Returns property help shared by completion and hover.
///
/// # Arguments
///
/// * `property` — Accepted property spelling.
/// * `category` — Expected compiler value category.
/// * `builder` — Canonical builder name shared by aliases.
///
/// # Returns
///
/// A Markdown description with the expected type and a compilable example.
pub(super) fn property_documentation(property: &str, category: &str, builder: &str) -> String {
    let description = match builder {
        "foreground" => "Sets the text foreground color, including theme-backed colors.",
        "background" => "Sets the background color, including theme-backed colors.",
        "modifier" => "Applies text modifier flags; combine flags with `|`.",
        "borders" => "Selects border sides; combine sides with `|`.",
        "border_type" => "Selects the border drawing style.",
        "padding" => "Sets inner spacing in terminal cells.",
        "image_size" => "Sets image width and height in terminal cells.",
        "display" => "Selects block, flex, grid, or hidden layout.",
        "box_sizing" => "Chooses whether dimensions measure the content or border box.",
        "overflow" => "Controls overflowing content independently on both axes.",
        "size" => "Sets preferred width and height dimensions.",
        "min_size" => "Sets minimum width and height dimensions.",
        "max_size" => "Sets maximum width and height dimensions.",
        "aspect_ratio" => "Sets the preferred width-to-height ratio; use a positive finite number.",
        "margin" => "Sets outer edge spacing, including automatic margins.",
        "gap" => "Sets horizontal and vertical spacing between children.",
        "flex_direction" => "Selects the main axis and direction of flex children.",
        "flex_wrap" => "Controls whether flex children wrap onto additional lines.",
        "flex_basis" => "Sets the initial main-axis dimension of a flex item.",
        "flex_grow" => "Sets the nonnegative share of extra flex space assigned to this item.",
        "flex_shrink" => "Sets the nonnegative shrink factor when flex space is insufficient.",
        "align_items" => "Aligns children on the cross axis.",
        "align_self" => "Overrides cross-axis alignment for this item.",
        "align_content" => "Distributes flex lines or grid tracks on the cross axis.",
        "justify_items" => "Aligns grid children on the inline axis.",
        "justify_self" => "Overrides inline-axis alignment for this grid item.",
        "justify_content" => "Distributes children on the main or inline axis.",
        "grid_auto_flow" => "Chooses the direction and density of automatic grid placement.",
        "grid_template_rows" | "grid_template_columns" => {
            "Defines explicit grid tracks, including repeated fragments."
        }
        "grid_auto_rows" | "grid_auto_columns" => {
            "Defines sizing for automatically created grid tracks."
        }
        "grid_row" | "grid_column" => {
            "Sets start and end grid placement using signed lines or track spans."
        }
        "position" => "Selects normal, relative, absolute, fixed, or sticky positioning.",
        "inset" => "Sets top, right, bottom, and left offsets for positioned elements.",
        "z_index" => "Sets the stacking level of a positioned element.",
        _ => "Sets a style declaration.",
    };
    let example = templates(category)
        .iter()
        .find(|t| property != "aspect_ratio" || t.label != "0.0")
        .map(|t| render(&t.expression, false))
        .unwrap_or_default();
    format!(
        "`{property}: {};`\n\n{description} Supports `!important`.\n\n```rust\n{property}: {example};\n```",
        super::property_type(category)
    )
}
