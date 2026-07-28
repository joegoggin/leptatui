//! Routed keyboard interaction and rendering tests.
//!
//! Page-specific behavior is grouped by its primary route while cross-page
//! routing and responsive behavior remain in a focused routing module.
//!
//! # Modules
//!
//! - [`explorer`] — Explorer destination behavior.
//! - [`home`] — Home workflow and recent-file behavior.
//! - [`routing`] — Cross-page responsive behavior.
//! - [`viewer`] — Viewer editing, reload, diagnostics, and Markdown history.

use std::{
    cell::{Cell, RefCell},
    fs,
    rc::Rc,
};

use leptatui::prelude::{KeyCode, KeyControl, KeyEvent, KeyModifiers, View};
use ratatui::{Terminal, backend::TestBackend};

use crate::{
    controller::Controller, editor_process::EditorProcess, filesystem::FileSystem, ui::app_view,
};

use super::support::{TestTree, draw_editor, rendered_lines};

mod explorer;
mod home;
mod routing;
mod viewer;
