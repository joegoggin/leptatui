//! Routed page keyboard interaction and rendering tests.
//!
//! # Modules
//!
//! - [`explorer`] — Explorer destination behavior.
//! - [`home`] — Home workflow and recent-file behavior.
//! - [`viewer`] — Viewer editing, reload, diagnostics, and Markdown history.

use std::fs;

use leptatui::prelude::{KeyCode, KeyControl, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};

use super::support::{TestContexts, TestTree, draw_editor, rendered_lines};

mod explorer;
mod home;
mod viewer;
