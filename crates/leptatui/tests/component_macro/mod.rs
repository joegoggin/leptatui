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
    AnyView, AppControl, Borders, Children, Color, Display, FlexDirection, IntoView, KeyControl,
    Modifier, NavigateOptions, RenderCtx, Router, RouterProps, ThemeVariables, TuiStyle, View,
    button, component, div, dynamic, stylesheet, text, theme_color, use_key_event, view,
};
use leptos::prelude::{Get, ReadSignal, Update, signal};
use ratatui::{Terminal, backend::TestBackend};

use crate::support::{key, render_component, rendered_lines, rendered_text};

include!("context/mod.rs");
include!("error_handling/mod.rs");
include!("input/mod.rs");
include!("key_events/mod.rs");
include!("lifecycle/mod.rs");
include!("rendering/mod.rs");
include!("routing/mod.rs");
include!("styling/mod.rs");

include!("alias.rs");
include!("compile.rs");
