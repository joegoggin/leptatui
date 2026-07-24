//! Compile tests for Leptatui component macros.

use std::{
    cell::Cell,
    fs,
    path::{Path, PathBuf},
    process::Command,
    rc::Rc,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind,
};
use leptatui::context::provide_context;
use leptatui::{
    AnyView, AppControl, Children, Color, Display, IntoView, KeyControl, LayoutDirection,
    RenderCtx, Result, ThemeVariables, View, button, column, component, dynamic, row, stylesheet,
    text, theme_color, use_key_event, view,
};
use leptos::prelude::{GetUntracked, ReadSignal, Update, signal};
use ratatui::{Terminal, backend::TestBackend};

use crate::support::{key, render_component, rendered_text};

include!("context/mod.rs");
include!("input/mod.rs");
include!("key_events/mod.rs");
include!("lifecycle/mod.rs");
include!("rendering/mod.rs");
include!("routing/mod.rs");
include!("styling/mod.rs");

include!("alias.rs");
include!("compile.rs");
