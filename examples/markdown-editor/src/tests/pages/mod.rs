//! Routed page keyboard interaction and rendering tests.
//!
//! # Modules
//!
//! - [`explorer`] — Explorer destination behavior.
//! - [`home`] — Home workflow and recent-file behavior.
//! - [`viewer`] — Viewer editing, reload, diagnostics, and Markdown history.

use std::{
    cell::{Cell, RefCell},
    fs,
    rc::Rc,
};

use leptatui::prelude::{KeyCode, KeyControl, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};

use crate::{app::app_view, core::Controller, services::EditorProcess, services::FileSystem};

use super::support::{TestTree, draw_editor, rendered_lines};

mod explorer;
mod home;
mod viewer;
