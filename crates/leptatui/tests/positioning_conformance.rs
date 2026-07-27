//! Static, relative, absolute, fixed, and sticky positioning conformance tests.
//!
//! These tests exercise normal-flow participation, positioned subtree
//! translation, containing-block and scrollport selection, opposing edge
//! resolution, percentage insets, scrolling, and resize recomputation through
//! the public style API.
//!
//! # Modules
//!
//! - [`fixed`] — Viewport positioning, clipping, scrolling, and resizing.
//! - [`flow`] — Static, relative, and absolute flow behavior.
//! - [`interaction`] — Positioned focus, pointer, link, cursor, and wheel behavior.
//! - [`stacking`] — Z-index levels, source ordering, and stacking contexts.
//! - [`sticky`] — Sticky thresholds, scrollports, sizing, and focus constraints.
//! - [`support`] — Shared rendering and fixture helpers.

use crossterm::event::{Event, KeyModifiers, MouseEvent, MouseEventKind};
use leptatui::__private::FocusedControl;
use leptatui::prelude::*;
use ratatui::{Terminal, backend::TestBackend, layout::Rect};

#[path = "positioning_conformance/fixed.rs"]
mod fixed;
#[path = "positioning_conformance/flow.rs"]
mod flow;
#[path = "positioning_conformance/interaction.rs"]
mod interaction;
#[path = "positioning_conformance/stacking.rs"]
mod stacking;
#[path = "positioning_conformance/sticky.rs"]
mod sticky;
mod support;

use support::{
    draw_view, fixture_size, key, render_view, rendered_lines, rendered_text, retained_child_rects,
};

/// Creates physical inset edges from optional terminal-cell lengths.
///
/// # Arguments
///
/// * `top` — Optional inset from the top edge.
/// * `right` — Optional inset from the right edge.
/// * `bottom` — Optional inset from the bottom edge.
/// * `left` — Optional inset from the left edge.
///
/// # Returns
///
/// An [`Edges`] value containing definite or automatic inset lengths.
fn cell_insets(
    top: Option<f32>,
    right: Option<f32>,
    bottom: Option<f32>,
    left: Option<f32>,
) -> Edges<LengthAuto> {
    Edges::new(
        top.map_or(LengthAuto::Auto, |value| Length::cells(value).into()),
        right.map_or(LengthAuto::Auto, |value| Length::cells(value).into()),
        bottom.map_or(LengthAuto::Auto, |value| Length::cells(value).into()),
        left.map_or(LengthAuto::Auto, |value| Length::cells(value).into()),
    )
}

/// Returns one view's retained border box.
///
/// # Arguments
///
/// * `view` — Erased view whose retained geometry is inspected.
///
/// # Returns
///
/// A [`Rect`] containing the retained border-box geometry.
fn retained_rect(view: &AnyView) -> Rect {
    view.style_metadata()
        .and_then(StyleMetadata::layout_geometry)
        .expect("positioning fixture view should retain layout geometry")
        .border_box
}

/// Creates a mouse event at one absolute terminal coordinate.
///
/// # Arguments
///
/// * `kind` — Mouse action represented by the event.
/// * `column` — Zero-based terminal column.
/// * `row` — Zero-based terminal row.
///
/// # Returns
///
/// An [`Event`] containing the requested mouse action.
fn mouse(kind: MouseEventKind, column: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

/// Returns button focus states in logical source order.
///
/// # Arguments
///
/// * `view` — Root view whose logical descendants are inspected.
///
/// # Returns
///
/// A [`Vec`] containing one focus flag per button.
fn button_focuses(view: &dyn View) -> Vec<bool> {
    let mut focuses = Vec::new();
    collect_button_focuses(view, &mut focuses);
    focuses
}

/// Appends logical button focus states from one view subtree.
///
/// # Arguments
///
/// * `view` — Current view inspected for button metadata and children.
/// * `focuses` — Output vector receiving source-ordered focus flags.
fn collect_button_focuses(view: &dyn View, focuses: &mut Vec<bool>) {
    if let Some(button) = view.as_any().downcast_ref::<ButtonView>() {
        focuses.push(
            button
                .style_metadata()
                .expect("buttons should expose style metadata")
                .is_focused(),
        );
    }
    for child in view.children() {
        collect_button_focuses(child.as_view(), focuses);
    }
}
