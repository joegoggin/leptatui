//! Focusable rich-text URL and filesystem link.

use crate::{
    RichText,
    app::{AppControl, Result},
    component::{FocusedControl, RenderCtx},
    view::{LinkTarget, StyleMetadata, View, ViewType},
};

use crate::view::{
    core::{
        capabilities::impl_styled_view,
        render::{line_count_height, resolve_style, semantic_paragraph},
    },
    link::open_link_target,
};

/// Focusable standalone link view.
#[derive(Debug, PartialEq)]
pub struct LinkView {
    /// Rich label displayed by the link.
    pub(crate) label: RichText,
    /// URL, path, Markdown page, or inactive fragment target.
    pub(crate) target: LinkTarget,
    /// Selector, focus, and hit-test metadata.
    pub(crate) metadata: StyleMetadata,
}

/// Creates a focusable rich-text link.
///
/// Relative filesystem paths are resolved from the process working directory.
/// Empty and hash-prefixed fragment targets render without participating in
/// focus traversal.
///
/// # Arguments
///
/// * `label` — Rich text displayed by the link.
/// * `target` — URL, filesystem path, Markdown target, or fragment.
///
/// # Returns
///
/// A [`LinkView`] containing the resolved destination and fresh metadata.
pub fn link(label: impl Into<RichText>, target: impl Into<LinkTarget>) -> LinkView {
    let base = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    link_with_base(label, target, base)
}

/// Creates a rich-text link resolved against an explicit directory.
///
/// # Arguments
///
/// * `label` — Rich text displayed by the link.
/// * `target` — URL, filesystem path, Markdown target, or fragment.
/// * `base` — Directory used to resolve relative filesystem targets.
///
/// # Returns
///
/// A [`LinkView`] containing the resolved destination and fresh metadata.
pub(crate) fn link_with_base(
    label: impl Into<RichText>,
    target: impl Into<LinkTarget>,
    base: impl AsRef<std::path::Path>,
) -> LinkView {
    LinkView {
        label: label.into(),
        target: target.into().resolve_against(base),
        metadata: StyleMetadata::new(ViewType::Link),
    }
}

impl View for LinkView {
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> Result<()> {
        let style = resolve_style(&self.metadata, ctx);
        ctx.render_widget(semantic_paragraph(self.label.text(), style));
        ctx.record_metadata_hit_area(&self.metadata);
        self.metadata.clear_scroll_into_view_request();
        Ok(())
    }

    fn min_height(&self, ctx: &mut RenderCtx<'_, '_>) -> u16 {
        let style = resolve_style(&self.metadata, ctx);
        line_count_height(semantic_paragraph(self.label.text(), style).line_count(ctx.area().width))
    }

    fn style_metadata(&self) -> Option<&StyleMetadata> {
        Some(&self.metadata)
    }

    fn style_metadata_mut(&mut self) -> Option<&mut StyleMetadata> {
        Some(&mut self.metadata)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn can_reconcile_from(&self, previous: &dyn View) -> bool {
        previous
            .as_any()
            .downcast_ref::<Self>()
            .is_some_and(|previous| self.target == previous.target)
    }

    fn __focusable_count(&self) -> usize {
        usize::from(self.target.is_actionable())
    }

    fn __focused_index_inner(&self, index: &mut usize) -> Option<usize> {
        if !self.target.is_actionable() {
            return None;
        }
        let current = *index;
        *index = index.saturating_add(1);
        self.metadata.is_focused().then_some(current)
    }

    fn __set_focus_by_index_inner(&mut self, target: usize, index: &mut usize) {
        if !self.target.is_actionable() {
            return;
        }
        let focused = *index == target;
        self.metadata.set_focused(focused);
        if focused {
            self.metadata.request_scroll_into_view();
        } else {
            self.metadata.clear_scroll_into_view_request();
        }
        *index = index.saturating_add(1);
    }

    fn __focusable_index_at_position_inner(
        &self,
        column: u16,
        row: u16,
        index: &mut usize,
    ) -> Option<usize> {
        if !self.target.is_actionable() {
            return None;
        }
        let current = *index;
        *index = index.saturating_add(1);
        self.metadata
            .contains_hit_position(column, row)
            .then_some(current)
    }

    fn __focused_control_span(&self, ctx: &mut RenderCtx<'_, '_>) -> Option<(u32, u32)> {
        (self.metadata.is_focused() && self.metadata.scroll_into_view_requested())
            .then_some((0, u32::from(ctx.area().height)))
    }

    fn __activate_focused_button(&self) -> Result<Option<AppControl>> {
        if self.metadata.is_focused() && self.target.is_actionable() {
            return open_link_target(&self.target).map(Some);
        }
        Ok(None)
    }

    fn __focused_control(&self) -> Option<FocusedControl> {
        (self.metadata.is_focused() && self.target.is_actionable()).then_some(FocusedControl::Link)
    }

    fn __focused_link_target(&self) -> Option<LinkTarget> {
        (self.metadata.is_focused() && self.target.is_actionable()).then(|| self.target.clone())
    }
}

impl_styled_view!(LinkView);

impl crate::view::TextualView for LinkView {
    fn content(&self) -> &ratatui::text::Text<'static> {
        self.label.text()
    }
}

impl LinkView {
    /// Returns the visible rich-text label.
    ///
    /// # Returns
    ///
    /// A [`ratatui::text::Text`] reference containing the visible label.
    pub fn content(&self) -> &ratatui::text::Text<'static> {
        self.label.text()
    }

    /// Returns the classified destination activated by this link.
    ///
    /// # Returns
    ///
    /// A [`LinkTarget`] reference containing the classified destination.
    pub const fn target(&self) -> &LinkTarget {
        &self.target
    }
}
