//! Batched expression formatting and read-only discovery of project preferences.

use super::*;
use std::{
    io::Write,
    process::{Command, Stdio},
};

/// Reads project rustfmt preferences and applies editor indentation options.
///
/// # Arguments
///
/// * `path` — Source file used to locate project configuration.
/// * `lsp` — Editor formatting options.
///
/// # Returns
///
/// A complete option set, with conventional defaults when discovery fails.
pub(super) fn options(path: Option<&Path>, lsp: &Value) -> Options {
    let mut options = Options::default();
    if let Some(path) = path {
        if let Ok(output) = Command::new("rustfmt")
            .args(["--print-config", "current"])
            .arg(path)
            .output()
            && output.status.success()
        {
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                let Some((key, value)) = line.split_once('=') else {
                    continue;
                };
                match key.trim() {
                    "max_width" => options.width = value.trim().parse().unwrap_or(100),
                    "tab_spaces" => options.tab_size = value.trim().parse().unwrap_or(4),
                    "hard_tabs" => options.spaces = value.trim() != "true",
                    _ => {}
                }
            }
        }
        if let Some(manifest) = path
            .ancestors()
            .skip(1)
            .map(|p| p.join("Cargo.toml"))
            .find(|p| p.is_file())
            && let Ok(output) = Command::new("cargo")
                .args([
                    "metadata",
                    "--locked",
                    "--offline",
                    "--no-deps",
                    "--format-version",
                    "1",
                    "--manifest-path",
                ])
                .arg(manifest)
                .output()
            && output.status.success()
            && let Ok(metadata) = serde_json::from_slice::<Value>(&output.stdout)
            && let Some(packages) = metadata["packages"].as_array()
        {
            let package = packages
                .iter()
                .filter_map(|package| {
                    let root = Path::new(package["manifest_path"].as_str()?).parent()?;
                    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_owned());
                    let root = root.canonicalize().unwrap_or_else(|_| root.to_owned());
                    canonical
                        .starts_with(&root)
                        .then_some((root.components().count(), package))
                })
                .max_by_key(|(length, _)| *length);
            if let Some((_, package)) = package
                && let Some(edition) = package["edition"].as_str()
            {
                options.edition = edition.into();
            }
        }
    }
    if let Some(size) = lsp["tabSize"].as_u64().filter(|n| (1..=16).contains(n)) {
        options.tab_size = size as usize;
    }
    if let Some(spaces) = lsp["insertSpaces"].as_bool() {
        options.spaces = spaces;
    }
    options
}

/// Formats Rust expressions in one subprocess and retains values that cannot be formatted safely.
///
/// # Arguments
///
/// * `syntax` — Source document index.
/// * `macros` — Printable stylesheet invocations.
/// * `options` — Layout and Rust edition preferences.
/// * `path` — Document path for rustfmt configuration.
///
/// # Returns
///
/// A map of successfully formatted values indexed by statement byte offset.
pub(super) fn format(
    syntax: &Syntax<'_>,
    macros: &[&Macro],
    options: &Options,
    path: Option<&Path>,
) -> HashMap<usize, String> {
    let mut values = Vec::new();
    for m in macros {
        let tree = syntax.parse(m);
        let mut statements = Vec::new();
        tree.walk(&mut statements);
        for statement in statements {
            if let Some((span, _)) = value(syntax, statement) {
                let raw = syntax.text[span].trim();
                if !stylesheet_reference(raw) && syn::parse_str::<syn::Expr>(raw).is_ok() {
                    values.push((statement.span.start, raw));
                }
            }
        }
    }
    if values.is_empty() {
        return HashMap::new();
    }
    let source: String = values
        .iter()
        .enumerate()
        .map(|(i, (_, raw))| format!("fn __leptatui_value_{i}() {{ let __value = {raw}\n; }}\n"))
        .collect();
    let mut command = Command::new("rustfmt");
    command
        .args([
            "--emit",
            "stdout",
            "--edition",
            &options.edition,
            "--config",
        ])
        .arg(format!(
            "max_width={},tab_spaces={},hard_tabs={},newline_style=Unix",
            options.width, options.tab_size, !options.spaces
        ))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(config) = path.and_then(|path| {
        path.ancestors().skip(1).find_map(|directory| {
            [".rustfmt.toml", "rustfmt.toml"]
                .into_iter()
                .map(|name| directory.join(name))
                .find(|file| file.is_file())
        })
    }) {
        command.arg("--config-path").arg(config);
    }
    let Ok(mut child) = command.spawn() else {
        return HashMap::new();
    };
    if child
        .stdin
        .take()
        .unwrap()
        .write_all(source.as_bytes())
        .is_err()
    {
        let _ = child.kill();
        let _ = child.wait();
        return HashMap::new();
    }
    let Ok(output) = child.wait_with_output() else {
        return HashMap::new();
    };
    if !output.status.success() {
        return HashMap::new();
    }
    let Ok(text) = String::from_utf8(output.stdout) else {
        return HashMap::new();
    };
    let formatted = Syntax::new(&text);
    let mut results = HashMap::new();
    let mut i = 0;
    while i + 1 < formatted.tokens.len() {
        if formatted.at(i) != "fn" {
            i += 1;
            continue;
        }
        let Some(index) = formatted
            .at(i + 1)
            .strip_prefix("__leptatui_value_")
            .and_then(|n| n.parse::<usize>().ok())
        else {
            i += 1;
            continue;
        };
        let Some(&(key, original)) = values.get(index) else {
            i += 1;
            continue;
        };
        while i < formatted.tokens.len() && formatted.at(i) != "=" {
            i += 1;
        }
        if i == formatted.tokens.len() {
            break;
        }
        let start = formatted.tokens[i].span.end;
        i += 1;
        while i < formatted.tokens.len() && formatted.at(i) != ";" {
            if matches!(formatted.at(i), "{" | "(" | "[") {
                i = formatted.end(i, formatted.tokens.len());
            }
            i += 1;
        }
        let Some(end) = formatted.tokens.get(i).map(|t| t.span.start) else {
            break;
        };
        let expression = text[start..end].trim();
        // Rustfmt may relocate a comment after the synthetic semicolon. In that
        // case the original value is safer than reconstructing its attachment.
        let original_comments: Vec<_> = printer::comments(original)
            .into_iter()
            .map(|(_, c)| c)
            .collect();
        let new_comments: Vec<_> = printer::comments(expression)
            .into_iter()
            .map(|(_, c)| c)
            .collect();
        if original_comments == new_comments && literals(original) == literals(expression) {
            results.insert(
                key,
                printer::shift(expression, "", options.tab_size, options.tab_size),
            );
        }
        i += 1;
    }
    results
}

/// Returns literal spellings to protect multiline strings from reformatting changes.
///
/// # Arguments
///
/// * `text` — Rust expression.
///
/// # Returns
///
/// A list of literal token contents in source order.
fn literals(text: &str) -> Vec<&str> {
    let mut offset = 0;
    rustc_lexer::tokenize(text)
        .filter_map(|token| {
            let span = offset..offset + token.len;
            offset = span.end;
            matches!(token.kind, rustc_lexer::TokenKind::Literal { .. }).then(|| &text[span])
        })
        .collect()
}
