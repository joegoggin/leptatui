//! File-backed Markdown navigation, link resolution, and page history.

use std::{
    cell::RefCell,
    collections::HashMap,
    fmt, fs,
    path::{Path, PathBuf},
    rc::Rc,
};

use percent_encoding::percent_decode_str;
use pulldown_cmark::LinkType;

use crate::{
    AnyView, IntoView, LinkTarget, RichText, View,
    app::{AppControl, Result},
    component::{KeyControl, RenderCtx},
    paragraph,
};

use super::{MarkdownOptions, markdown_with_options_and_source};

/// Stateful, file-backed Markdown document boundary.
///
/// Previously visited pages stay in memory so back and forward navigation
/// restore their retained focus and scroll state.
#[derive(Clone)]
pub struct MarkdownView {
    state: Rc<RefCell<MarkdownState>>,
}

/// Mutable navigation state shared across reconciled Markdown boundaries.
#[derive(Debug, PartialEq)]
struct MarkdownState {
    root_path: PathBuf,
    options: MarkdownOptions,
    current: MarkdownPage,
    back: Vec<MarkdownPage>,
    forward: Vec<MarkdownPage>,
}

/// One cached page in a [`MarkdownView`] navigation history.
#[derive(Debug, PartialEq)]
struct MarkdownPage {
    path: PathBuf,
    document: AnyView,
}

impl MarkdownView {
    /// Creates a file-backed Markdown boundary rooted at `path`.
    pub(super) fn new(path: &Path, options: MarkdownOptions) -> Self {
        let root_path = absolute_path(path);
        let current = load_markdown_page(root_path.clone(), options, None);
        Self {
            state: Rc::new(RefCell::new(MarkdownState {
                root_path,
                options,
                current,
                back: Vec::new(),
                forward: Vec::new(),
            })),
        }
    }

    /// Returns the path of the currently displayed Markdown page.
    pub fn current_path(&self) -> PathBuf {
        self.state.borrow().current.path.clone()
    }

    /// Returns whether a cached page is available in back history.
    pub fn can_go_back(&self) -> bool {
        !self.state.borrow().back.is_empty()
    }

    /// Returns whether a cached page is available in forward history.
    pub fn can_go_forward(&self) -> bool {
        !self.state.borrow().forward.is_empty()
    }

    /// Returns whether this boundary belongs to the same declarative root.
    fn can_reconcile_from(&self, previous: &Self) -> bool {
        let state = self.state.borrow();
        let previous = previous.state.borrow();
        state.root_path == previous.root_path && state.options == previous.options
    }
}

impl MarkdownState {
    /// Navigates to the focused in-app Markdown target, if one exists.
    fn navigate_focused_link(&mut self) -> bool {
        let Some(LinkTarget::Markdown { path, fragment }) =
            self.current.document.__focused_link_target()
        else {
            return false;
        };

        let next = load_markdown_page(path, self.options, fragment.as_deref());
        let previous = std::mem::replace(&mut self.current, next);
        self.back.push(previous);
        self.forward.clear();
        true
    }

    /// Restores the most recent cached page from back history.
    fn go_back(&mut self) -> bool {
        let Some(previous) = self.back.pop() else {
            return false;
        };
        let current = std::mem::replace(&mut self.current, previous);
        self.forward.push(current);
        true
    }

    /// Restores the next cached page from forward history.
    fn go_forward(&mut self) -> bool {
        let Some(next) = self.forward.pop() else {
            return false;
        };
        let current = std::mem::replace(&mut self.current, next);
        self.back.push(current);
        true
    }
}

impl fmt::Debug for MarkdownView {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MarkdownView")
            .field("state", &self.state.borrow())
            .finish()
    }
}

impl PartialEq for MarkdownView {
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state
    }
}

impl View for MarkdownView {
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        self.state.borrow().current.document.render(ctx)
    }

    fn min_height(&self, ctx: &mut RenderCtx<'_, '_>) -> u16 {
        self.state.borrow().current.document.__min_height(ctx)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn reconcile(&mut self, previous: &dyn View) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>()
            && self.can_reconcile_from(previous)
        {
            self.state = previous.state.clone();
        }
    }

    fn can_reconcile_from(&self, previous: &dyn View) -> bool {
        previous
            .as_any()
            .downcast_ref::<Self>()
            .is_some_and(|previous| self.can_reconcile_from(previous))
    }

    fn __dispatch_event(&mut self, event: &crossterm::event::Event) -> Result<AppControl> {
        self.state
            .borrow_mut()
            .current
            .document
            .__dispatch_event(event)
    }

    fn __dispatch_key_event(&mut self, key: crossterm::event::KeyEvent) -> Result<KeyControl> {
        self.state
            .borrow_mut()
            .current
            .document
            .__dispatch_key_event(key)
    }

    fn __flush_pending_input(&mut self) -> Option<AppControl> {
        self.state
            .borrow_mut()
            .current
            .document
            .__flush_pending_input()
    }

    fn __focusable_count(&self) -> usize {
        self.state.borrow().current.document.__focusable_count()
    }

    fn __focused_index_inner(&self, index: &mut usize) -> Option<usize> {
        self.state
            .borrow()
            .current
            .document
            .__focused_index_inner(index)
    }

    fn __set_focus_by_index_inner(&mut self, target: usize, index: &mut usize) {
        self.state
            .borrow_mut()
            .current
            .document
            .__set_focus_by_index_inner(target, index);
    }

    fn __focusable_index_at_position_inner(
        &self,
        column: u16,
        row: u16,
        index: &mut usize,
    ) -> Option<usize> {
        self.state
            .borrow()
            .current
            .document
            .__focusable_index_at_position_inner(column, row, index)
    }

    fn __focused_control_span(&self, ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        self.state
            .borrow()
            .current
            .document
            .__focused_button_span(ctx)
    }

    fn __activate_focused_button(&self) -> Result<Option<AppControl>> {
        let mut state = self.state.borrow_mut();
        if state.navigate_focused_link() {
            return Ok(Some(AppControl::Continue));
        }
        state.current.document.__activate_focused_button()
    }

    fn __handle_focused_input_key(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> Option<KeyControl> {
        self.state
            .borrow_mut()
            .current
            .document
            .__handle_focused_input_key(key)
    }

    fn __focused_control(&self) -> Option<crate::component::FocusedControl> {
        self.state.borrow().current.document.__focused_control()
    }

    fn __handle_form_key(&mut self, key: crossterm::event::KeyEvent) -> Option<KeyControl> {
        self.state
            .borrow_mut()
            .current
            .document
            .__handle_form_key(key)
    }

    fn __scroll_first_overflowing(&mut self, delta: i16) -> bool {
        self.state
            .borrow_mut()
            .current
            .document
            .__scroll_first_overflowing(delta)
    }

    fn __scroll_first_overflowing_to_top(&mut self) -> bool {
        self.state
            .borrow_mut()
            .current
            .document
            .__scroll_first_overflowing_to_top()
    }

    fn __scroll_first_overflowing_to_bottom(&mut self) -> bool {
        self.state
            .borrow_mut()
            .current
            .document
            .__scroll_first_overflowing_to_bottom()
    }

    fn __has_overflowing_scroll_target(&self) -> bool {
        self.state
            .borrow()
            .current
            .document
            .__has_overflowing_scroll_target()
    }

    fn __scroll_overflowing_at_position(&mut self, column: u16, row: u16, delta: i16) -> bool {
        self.state
            .borrow_mut()
            .current
            .document
            .__scroll_overflowing_at_position(column, row, delta)
    }

    fn __set_scroll_to_top_key_pending(&self, pending: bool) -> bool {
        self.state
            .borrow()
            .current
            .document
            .__set_scroll_to_top_key_pending(pending)
    }

    fn __take_scroll_to_top_key_pending(&self) -> bool {
        self.state
            .borrow()
            .current
            .document
            .__take_scroll_to_top_key_pending()
    }

    fn __focused_link_target(&self) -> Option<LinkTarget> {
        self.state.borrow().current.document.__focused_link_target()
    }

    fn __request_scroll_to_id(&mut self, id: &str) -> bool {
        self.state
            .borrow_mut()
            .current
            .document
            .__request_scroll_to_id(id)
    }

    fn __has_scroll_to_anchor_request(&self) -> bool {
        self.state
            .borrow()
            .current
            .document
            .__has_scroll_to_anchor_request()
    }

    fn __navigate_markdown_history(&mut self, back: bool) -> bool {
        let mut state = self.state.borrow_mut();
        let moved = if back {
            state.go_back()
        } else {
            state.go_forward()
        };
        moved || state.current.document.__navigate_markdown_history(back)
    }

    fn __clear_hit_areas(&self) {
        self.state.borrow().current.document.__clear_hit_areas();
    }
}

/// Per-document context used while parsing links and heading anchors.
pub(super) struct MarkdownParseContext<'a> {
    link_base: &'a Path,
    source_path: Option<&'a Path>,
    heading_counts: HashMap<String, usize>,
}

impl<'a> MarkdownParseContext<'a> {
    /// Creates parsing context for in-memory or file-backed Markdown.
    pub(super) fn new(link_base: &'a Path, source_path: Option<&'a Path>) -> Self {
        Self {
            link_base,
            source_path,
            heading_counts: HashMap::new(),
        }
    }

    /// Returns whether headings should receive file-navigation anchors.
    pub(super) const fn has_source_path(&self) -> bool {
        self.source_path.is_some()
    }

    /// Returns the unique GitHub-style slug for one heading.
    pub(super) fn heading_slug(&mut self, content: &RichText) -> String {
        let visible = content
            .text()
            .lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect::<String>();
        let base = github_heading_slug(&visible);
        let count = self.heading_counts.entry(base.clone()).or_default();
        let slug = if *count == 0 {
            base
        } else {
            format!("{base}-{count}")
        };
        *count = count.saturating_add(1);
        slug
    }

    /// Classifies a parsed Markdown link for this document boundary.
    pub(super) fn link_target(&self, link_type: LinkType, destination: &str) -> LinkTarget {
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
            if path.is_empty() {
                if let Some(fragment) = fragment.filter(|fragment| !fragment.is_empty()) {
                    return LinkTarget::Markdown {
                        path: source_path.to_path_buf(),
                        fragment: Some(fragment.to_owned()),
                    };
                }
            } else if is_markdown_path(Path::new(path)) {
                return LinkTarget::Markdown {
                    path: absolute_path_from(Path::new(path), self.link_base),
                    fragment: fragment
                        .filter(|fragment| !fragment.is_empty())
                        .map(str::to_owned),
                };
            }
        }

        ordinary.resolve_against(self.link_base)
    }
}

/// Returns an absolute path without requiring the target to exist.
fn absolute_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        let base = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        base.join(path)
    }
}

/// Resolves `path` against `base` without requiring the target to exist.
fn absolute_path_from(path: &Path, base: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

/// Returns whether a local path names a supported Markdown file extension.
fn is_markdown_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown")
        })
}

/// Produces the base anchor used for GitHub-style heading fragments.
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
fn normalized_fragment(fragment: &str) -> String {
    percent_decode_str(fragment)
        .decode_utf8_lossy()
        .chars()
        .flat_map(char::to_lowercase)
        .collect()
}

/// Loads one page, retaining read failures as navigable in-app content.
fn load_markdown_page(
    path: PathBuf,
    options: MarkdownOptions,
    fragment: Option<&str>,
) -> MarkdownPage {
    let mut document = match fs::read_to_string(&path) {
        Ok(source) => {
            let link_base = path.parent().unwrap_or_else(|| Path::new("."));
            markdown_with_options_and_source(&source, options, link_base, Some(&path))
        }
        Err(error) => crate::column([paragraph(format!(
            "failed to read Markdown file `{}`: {error}",
            path.display()
        ))])
        .into_view(),
    };

    if let Some(fragment) = fragment {
        document.__request_scroll_to_id(&normalized_fragment(fragment));
    }

    MarkdownPage { path, document }
}
