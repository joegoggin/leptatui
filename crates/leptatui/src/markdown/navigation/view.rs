//! Runtime file-backed Markdown view state and page history.

use std::{
    cell::RefCell,
    fmt, fs,
    path::{Path, PathBuf},
    rc::Rc,
};

use crate::{
    AnyView, AvailableSpace, IntoView, LayoutSize, LinkTarget, View,
    app::{AppControl, Result},
    component::{KeyControl, RenderCtx},
    paragraph,
};

use super::{
    super::{MarkdownOptions, markdown_with_options_and_source},
    parse::{absolute_path, normalized_fragment},
};

/// Stateful, file-backed Markdown document boundary.
///
/// Previously visited pages stay in memory so back and forward navigation
/// restore their retained focus and scroll state.
#[derive(Clone)]
pub struct MarkdownView {
    /// Shared navigation state retained across reconciled view boundaries.
    state: Rc<RefCell<MarkdownState>>,
}

/// Mutable navigation state shared across reconciled Markdown boundaries.
#[derive(Debug, PartialEq)]
struct MarkdownState {
    /// Absolute path of the declarative root used for reconciliation identity.
    root_path: PathBuf,
    /// Rendering options applied to every page loaded by this boundary.
    options: MarkdownOptions,
    /// Page currently displayed by the boundary.
    current: MarkdownPage,
    /// Previously visited pages ordered from oldest to newest.
    back: Vec<MarkdownPage>,
    /// Forward-history pages ordered from farthest to nearest.
    forward: Vec<MarkdownPage>,
}

/// One cached page in a [`MarkdownView`] navigation history.
#[derive(Debug, PartialEq)]
struct MarkdownPage {
    /// Absolute or working-directory-relative path represented by the page.
    path: PathBuf,
    /// Parsed document or rendered file-read failure retained for the page.
    document: AnyView,
}

impl MarkdownView {
    /// Creates a file-backed Markdown boundary rooted at `path`.
    ///
    /// The initial page is loaded immediately. Read failures are retained as
    /// navigable in-app content rather than returned as errors.
    ///
    /// # Arguments
    ///
    /// * `path` — Markdown file that identifies the declarative root.
    /// * `options` — Rendering options applied to every loaded page.
    ///
    /// # Returns
    ///
    /// A [`MarkdownView`] displaying the initial page or its read failure.
    pub(in crate::markdown) fn new(path: &Path, options: MarkdownOptions) -> Self {
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
    ///
    /// # Returns
    ///
    /// An owned [`PathBuf`] containing the current page path.
    pub fn current_path(&self) -> PathBuf {
        self.state.borrow().current.path.clone()
    }

    /// Returns whether a cached page is available in back history.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether backward navigation can restore a page.
    pub fn can_go_back(&self) -> bool {
        !self.state.borrow().back.is_empty()
    }

    /// Returns whether a cached page is available in forward history.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether forward navigation can restore a page.
    pub fn can_go_forward(&self) -> bool {
        !self.state.borrow().forward.is_empty()
    }

    /// Returns whether this boundary belongs to the same declarative root.
    ///
    /// # Arguments
    ///
    /// * `previous` — Earlier Markdown boundary considered for reconciliation.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether root path and rendering options match.
    fn can_reconcile_from(&self, previous: &Self) -> bool {
        let state = self.state.borrow();
        let previous = previous.state.borrow();
        state.root_path == previous.root_path && state.options == previous.options
    }
}

impl MarkdownState {
    /// Navigates to the focused in-app Markdown target, if one exists.
    ///
    /// Successful navigation pushes the current page into back history and
    /// clears forward history.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether a focused Markdown target was opened.
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
    ///
    /// # Returns
    ///
    /// A boolean indicating whether backward navigation restored a page.
    fn go_back(&mut self) -> bool {
        let Some(previous) = self.back.pop() else {
            return false;
        };
        let current = std::mem::replace(&mut self.current, previous);
        self.forward.push(current);
        true
    }

    /// Restores the next cached page from forward history.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether forward navigation restored a page.
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

    fn measure(
        &self,
        known_dimensions: LayoutSize<Option<f32>>,
        available_space: LayoutSize<AvailableSpace>,
        ctx: &mut RenderCtx<'_, '_>,
    ) -> LayoutSize<f32> {
        self.state
            .borrow()
            .current
            .document
            .measure(known_dimensions, available_space, ctx)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn __visit_layout_children(
        &self,
        ctx: &mut RenderCtx<'_, '_>,
        visitor: &mut dyn FnMut(&AnyView, &mut RenderCtx<'_, '_>),
    ) {
        visitor(&self.state.borrow().current.document, ctx);
    }

    fn __is_layout_transparent(&self) -> bool {
        true
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

    fn __scroll_first_overflowing(&mut self, delta: crate::Axes<i16>) -> bool {
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

    fn __scroll_overflowing_at_position(
        &mut self,
        column: u16,
        row: u16,
        delta: crate::Axes<i16>,
    ) -> bool {
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

/// Loads one page, retaining read failures as navigable in-app content.
///
/// # Arguments
///
/// * `path` — Markdown file path represented by the loaded page.
/// * `options` — Rendering options applied while parsing the file.
/// * `fragment` — Optional heading fragment requested after parsing.
///
/// # Returns
///
/// A [`MarkdownPage`] containing parsed content or a rendered read failure.
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
        Err(error) => crate::div([paragraph(format!(
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
