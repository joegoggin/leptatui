//! Parse-time Markdown link resolution and heading-anchor generation.

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use percent_encoding::percent_decode_str;
use pulldown_cmark::LinkType;

use crate::{LinkTarget, RichText};

/// Per-document context used while parsing links and heading anchors.
pub(in crate::markdown) struct MarkdownParseContext<'a> {
    /// Directory used to resolve relative local link targets.
    link_base: &'a Path,
    /// Current file path when parsing file-backed Markdown.
    source_path: Option<&'a Path>,
    /// Next numeric suffix to try for each base heading slug.
    heading_counts: HashMap<String, usize>,
    /// Every heading slug already assigned in the current document.
    heading_slugs: HashSet<String>,
}

impl<'a> MarkdownParseContext<'a> {
    /// Creates parsing context for in-memory or file-backed Markdown.
    ///
    /// # Arguments
    ///
    /// * `link_base` — Directory used to resolve relative local targets.
    /// * `source_path` — Current Markdown file path, when parsing from disk.
    ///
    /// # Returns
    ///
    /// A [`MarkdownParseContext`] with empty heading-slug state.
    pub(in crate::markdown) fn new(link_base: &'a Path, source_path: Option<&'a Path>) -> Self {
        Self {
            link_base,
            source_path,
            heading_counts: HashMap::new(),
            heading_slugs: HashSet::new(),
        }
    }

    /// Returns whether headings should receive file-navigation anchors.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the context has a source file path.
    pub(in crate::markdown) const fn has_source_path(&self) -> bool {
        self.source_path.is_some()
    }

    /// Returns the unique GitHub-style slug for one heading.
    ///
    /// Previously assigned slugs are retained so duplicate headings receive
    /// collision-free numeric suffixes.
    ///
    /// # Arguments
    ///
    /// * `content` — Rich heading content whose visible text forms the slug.
    ///
    /// # Returns
    ///
    /// A unique [`String`] containing the normalized heading slug.
    pub(in crate::markdown) fn heading_slug(&mut self, content: &RichText) -> String {
        let visible = content
            .text()
            .lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect::<String>();
        let base = github_heading_slug(&visible);
        let next = self.heading_counts.entry(base.clone()).or_default();

        loop {
            let slug = if *next == 0 {
                base.clone()
            } else {
                format!("{base}-{next}")
            };
            *next = next.saturating_add(1);

            if self.heading_slugs.insert(slug.clone()) {
                return slug;
            }
        }
    }

    /// Classifies a parsed Markdown link for this document boundary.
    ///
    /// # Arguments
    ///
    /// * `link_type` — Link classification emitted by the Markdown parser.
    /// * `destination` — Raw, potentially percent-encoded destination.
    ///
    /// # Returns
    ///
    /// A [`LinkTarget`] resolved relative to this document's link base.
    pub(in crate::markdown) fn link_target(
        &self,
        link_type: LinkType,
        destination: &str,
    ) -> LinkTarget {
        if link_type == LinkType::Email && !destination.starts_with("mailto:") {
            return LinkTarget::Url(format!("mailto:{destination}"));
        }

        let ordinary = LinkTarget::from(destination);
        if matches!(ordinary, LinkTarget::Url(_)) {
            return ordinary;
        }

        if let Some(source_path) = self.source_path {
            let (path, fragment) = destination
                .split_once('#')
                .map_or((destination, None), |(path, fragment)| {
                    (path, Some(fragment))
                });
            let decoded_path = percent_decode_str(path).decode_utf8_lossy();
            if path.is_empty() {
                if let Some(fragment) = fragment.filter(|fragment| !fragment.is_empty()) {
                    return LinkTarget::Markdown {
                        path: source_path.to_path_buf(),
                        fragment: Some(fragment.to_owned()),
                    };
                }
            } else if is_markdown_path(Path::new(decoded_path.as_ref())) {
                return LinkTarget::Markdown {
                    path: absolute_path_from(Path::new(decoded_path.as_ref()), self.link_base),
                    fragment: fragment
                        .filter(|fragment| !fragment.is_empty())
                        .map(str::to_owned),
                };
            }
        }

        LinkTarget::from(
            percent_decode_str(destination)
                .decode_utf8_lossy()
                .into_owned(),
        )
        .resolve_against(self.link_base)
    }
}

/// Returns an absolute path without requiring the target to exist.
///
/// Relative paths are joined to the process working directory, or `.` when
/// the working directory cannot be read.
///
/// # Arguments
///
/// * `path` — Absolute or working-directory-relative path to normalize.
///
/// # Returns
///
/// A [`PathBuf`] containing the absolute or joined path.
pub(super) fn absolute_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        let base = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        base.join(path)
    }
}

/// Resolves `path` against `base` without requiring the target to exist.
///
/// # Arguments
///
/// * `path` — Absolute or base-relative path to resolve.
/// * `base` — Directory joined to relative paths.
///
/// # Returns
///
/// A [`PathBuf`] containing the absolute path or base-relative result.
fn absolute_path_from(path: &Path, base: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

/// Returns whether a local path names a supported Markdown file extension.
///
/// # Arguments
///
/// * `path` — Local path whose extension should be inspected.
///
/// # Returns
///
/// A boolean indicating whether the extension is `md` or `markdown`, ignoring
/// ASCII case.
fn is_markdown_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown")
        })
}

/// Produces the base anchor used for GitHub-style heading fragments.
///
/// # Arguments
///
/// * `heading` — Visible heading text to normalize.
///
/// # Returns
///
/// A [`String`] containing the lowercase base slug before collision handling.
fn github_heading_slug(heading: &str) -> String {
    let mut slug = String::new();
    for character in heading.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() || character == '-' || character == '_' {
            slug.push(character);
        } else if character.is_whitespace() {
            slug.push('-');
        }
    }
    slug
}

/// Normalizes a percent-encoded fragment for heading-id comparison.
///
/// # Arguments
///
/// * `fragment` — Raw, potentially percent-encoded fragment identifier.
///
/// # Returns
///
/// A lowercase [`String`] containing the decoded fragment.
pub(super) fn normalized_fragment(fragment: &str) -> String {
    percent_decode_str(fragment)
        .decode_utf8_lossy()
        .chars()
        .flat_map(char::to_lowercase)
        .collect()
}
