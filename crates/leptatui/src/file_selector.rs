//! Runner-owned interactive file selection.
//!
//! The [`use_file_selector`] hook creates a stateful handle that opens a
//! style-isolated, full-terminal picker and stores the canonical path selected
//! by the user.

use std::{
    cmp::Ordering,
    ffi::{OsStr, OsString},
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering as AtomicOrdering},
    },
    time::Duration,
};

use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use leptos::prelude::{
    ArcReadSignal, ArcRwSignal, ArcWriteSignal, Get, GetUntracked, Set, Update, With,
    WithUntracked, arc_signal,
};
use ratatui::layout::Alignment;

use crate::{
    Action, AppControl, Axes, BorderType, Borders, BoxSizing, Color, Dimension, Display, Edges,
    FlexDirection, Fraction, GridTemplateTrack, GridTrackSize, IntoView, KeyControl, LayoutSize,
    Length, LengthAuto, Modifier, Overflow, Position, TuiSpacing, ZIndex,
    app::{StandaloneScreenRegistry, request_redraw},
    button, context, div,
    file_system::{FileEntry, FileKind, FileOperation, FileSystem, use_file_system},
    image, keyed, text, use_key_event, view,
    view::{
        ComponentView,
        content::code_block::{PreparedCodeBlock, code_block_from_prepared, prepare_code_block},
    },
};

/// Maximum byte prefix loaded into a regular-file preview.
const MAX_TEXT_PREVIEW_BYTES: u64 = 8 * 1024;

/// Maximum logical lines retained in a regular-file preview.
const MAX_TEXT_PREVIEW_LINES: usize = 64;

/// Delay that lets rapid selection changes supersede preview work.
const PREVIEW_DEBOUNCE: Duration = Duration::from_millis(50);

/// Cloneable handle for opening a file selector and reading its stored file.
#[derive(Clone)]
pub struct FileSelector {
    /// Runner that owns the selector while it is open.
    screens: Option<StandaloneScreenRegistry>,
    /// Selected-file value retained by a signal-backed handle.
    file: ArcReadSignal<Option<PathBuf>>,
    /// Setter for the selected-file value.
    set_file: ArcWriteSignal<Option<PathBuf>>,
}

impl FileSelector {
    /// Opens a selector in the process current directory for any regular file.
    pub fn select(&self) {
        self.select_with_options(FileSelectorOptions::new());
    }

    /// Opens a selector using the provided options.
    ///
    /// Opening or cancelling does not change the previously stored file.
    ///
    /// # Arguments
    ///
    /// * `options` — Starting-directory and file-filter configuration.
    pub fn select_with_options(&self, options: FileSelectorOptions) {
        let Some(screens) = self.screens.clone() else {
            return;
        };
        let requested = options
            .starting_directory
            .clone()
            .map_or_else(std::env::current_dir, Ok);
        let initial_directory = requested
            .and_then(fs::canonicalize)
            .and_then(|path| {
                if path.is_dir() {
                    Ok(path)
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::NotADirectory,
                        "file selector starting path is not a directory",
                    ))
                }
            })
            .map_err(|error| error.to_string());
        let screen = ComponentView::new_style_isolated(FileSelectorScreen::with_props(
            FileSelectorScreenProps {
                initial_directory,
                options,
                set_file: self.set_file.clone(),
                screens: screens.clone(),
            },
        ));
        screens.register_owned(screen);
        request_redraw();
    }

    /// Clears the stored file.
    pub fn clear(&self) {
        let _ = self.set_file.try_set(None);
        request_redraw();
    }

    /// Reactively returns the stored file.
    ///
    /// Calling this method inside an effect or reactive closure subscribes it
    /// to successful selections and calls to [`Self::clear`].
    ///
    /// # Returns
    ///
    /// An optional canonical [`PathBuf`] selected by the user.
    pub fn get_file(&self) -> Option<PathBuf> {
        self.file.get()
    }
}

/// Options applied to one file-selector session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileSelectorOptions {
    /// Directory displayed when the selector opens.
    starting_directory: Option<PathBuf>,
    /// Case-insensitive extension allowlist without leading dots.
    extensions: Vec<String>,
    /// Whether names beginning with a dot are visible.
    show_hidden: bool,
}

impl FileSelectorOptions {
    /// Creates options that start in the current directory and show all files.
    ///
    /// # Returns
    ///
    /// A default [`FileSelectorOptions`] value with hidden files visible.
    pub fn new() -> Self {
        Self {
            starting_directory: None,
            extensions: Vec::new(),
            show_hidden: true,
        }
    }

    /// Sets the directory displayed when the selector opens.
    ///
    /// # Arguments
    ///
    /// * `directory` — Existing directory to canonicalize when opened.
    ///
    /// # Returns
    ///
    /// A configured [`FileSelectorOptions`] value.
    pub fn starting_directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.starting_directory = Some(directory.into());
        self
    }

    /// Sets the regular-file extensions permitted for selection.
    ///
    /// Extensions are matched case-insensitively and may include a leading
    /// dot. Directories remain visible and an empty iterator permits all files.
    ///
    /// # Arguments
    ///
    /// * `extensions` — Extension names accepted by the selector.
    ///
    /// # Returns
    ///
    /// A configured [`FileSelectorOptions`] value.
    pub fn extensions<I, S>(mut self, extensions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.extensions = extensions
            .into_iter()
            .map(|extension| {
                extension
                    .as_ref()
                    .trim_start_matches('.')
                    .to_ascii_lowercase()
            })
            .filter(|extension| !extension.is_empty())
            .collect();
        self.extensions.sort();
        self.extensions.dedup();
        self
    }

    /// Configures whether hidden names are visible initially.
    ///
    /// # Arguments
    ///
    /// * `show_hidden` — Whether entries beginning with a dot are visible.
    ///
    /// # Returns
    ///
    /// A configured [`FileSelectorOptions`] value.
    pub const fn show_hidden(mut self, show_hidden: bool) -> Self {
        self.show_hidden = show_hidden;
        self
    }

    /// Returns whether a regular file is permitted by the extension filter.
    ///
    /// # Arguments
    ///
    /// * `path` — File path whose extension should be tested.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the file is selectable.
    fn permits(&self, path: &Path) -> bool {
        self.extensions.is_empty()
            || path
                .extension()
                .and_then(OsStr::to_str)
                .is_some_and(|extension| {
                    self.extensions
                        .iter()
                        .any(|allowed| extension.eq_ignore_ascii_case(allowed))
                })
    }
}

impl Default for FileSelectorOptions {
    /// Returns the default selector options.
    fn default() -> Self {
        Self::new()
    }
}

/// Creates a file-selector handle for the current managed application.
///
/// # Returns
///
/// A [`FileSelector`] with no stored file.
///
pub fn use_file_selector() -> FileSelector {
    let screens = context::use_context::<StandaloneScreenRegistry>();
    let (file, set_file) = arc_signal(None);
    FileSelector {
        screens,
        file,
        set_file,
    }
}

/// Preview content derived from the highlighted filesystem entry.
#[derive(Clone, Debug, PartialEq)]
enum PreviewContent {
    /// Structured entries rendered with selector entry colors.
    Directory(Vec<DirectoryPreviewEntry>),
    /// A supported image path rendered by Leptatui.
    Image(PathBuf),
    /// Syntax-highlighted UTF-8 source prepared outside the render thread.
    Source(PreparedCodeBlock),
    /// Plain preview text or metadata.
    Text(String),
}

/// One generation-scoped request for selected-entry preview content.
#[derive(Clone)]
struct PreviewRequest {
    /// Filesystem entry selected for preview.
    entry: FileEntry,
    /// Generation assigned when this request was dispatched.
    generation: u64,
    /// Latest selector generation shared across preview requests.
    latest_generation: Arc<AtomicU64>,
}

impl PreviewRequest {
    /// Returns whether this request still targets the latest selection.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether no newer preview request superseded this one.
    fn is_current(&self) -> bool {
        self.latest_generation.load(AtomicOrdering::Acquire) == self.generation
    }
}

/// Filesystem entry metadata retained while formatting directory previews.
#[derive(Clone, Debug, Eq, PartialEq)]
struct DirectoryPreviewEntry {
    /// Visible directory-entry name.
    name: OsString,
    /// Classification of the followed entry target.
    kind: FileKind,
    /// Whether the visible entry is a symbolic link.
    symlink: bool,
}

/// Structured content rendered in the parent-directory pane.
#[derive(Clone, Debug, Eq, PartialEq)]
enum ParentPreviewContent {
    /// Sorted filesystem rows retaining the current-directory marker.
    Entries(Vec<ParentPreviewEntry>),
    /// Plain fallback or error text.
    Text(String),
}

/// Filesystem entry metadata retained for one parent-directory row.
#[derive(Clone, Debug, Eq, PartialEq)]
struct ParentPreviewEntry {
    /// Entry rendered on this row.
    entry: DirectoryPreviewEntry,
    /// Whether this entry represents the current directory.
    current: bool,
}

impl ParentPreviewEntry {
    /// Returns this parent entry's marker and icon-prefixed display label.
    ///
    /// # Returns
    ///
    /// A [`String`] whose leading marker identifies the current directory.
    fn label(&self) -> String {
        let marker = if self.current { ">" } else { " " };
        format!("{marker} {}", self.entry.label())
    }
}

impl DirectoryPreviewEntry {
    /// Creates preview metadata from a directory entry.
    ///
    /// Unreadable or missing targets remain visible as [`FileKind::Other`].
    ///
    /// # Arguments
    ///
    /// * `entry` — Directory entry whose visible and target metadata is inspected.
    ///
    /// # Returns
    ///
    /// A [`DirectoryPreviewEntry`] retaining its name, target kind, and symlink state.
    fn from_dir_entry(entry: fs::DirEntry) -> Self {
        let path = entry.path();
        let symlink = entry
            .file_type()
            .is_ok_and(|file_type| file_type.is_symlink());
        let kind = fs::metadata(path).map_or(FileKind::Other, |metadata| {
            if metadata.is_dir() {
                FileKind::Directory
            } else if metadata.is_file() {
                FileKind::File
            } else {
                FileKind::Other
            }
        });
        Self {
            name: entry.file_name(),
            kind,
            symlink,
        }
    }

    /// Returns this preview entry's icon-prefixed display label.
    ///
    /// # Returns
    ///
    /// A [`String`] containing the shared selector entry format.
    fn label(&self) -> String {
        format_entry_label(&self.name, self.kind, self.symlink)
    }
}

/// Creates the full-terminal file-selector component.
///
/// # Arguments
///
/// * `initial_directory` — Canonical initial directory or initialization error.
/// * `options` — Filtering and visibility settings for this session.
/// * `set_file` — Setter receiving a successful selection.
/// * `screens` — Runner registry dismissed on selection or cancellation.
///
/// # Returns
///
/// A style-isolated file-selector view.
#[crate::component]
#[allow(non_snake_case)]
fn FileSelectorScreen(
    initial_directory: Result<PathBuf, String>,
    options: FileSelectorOptions,
    set_file: ArcWriteSignal<Option<PathBuf>>,
    screens: StandaloneScreenRegistry,
) -> impl IntoView {
    crate::stylesheet! {
        .file-selector => {
            box_sizing: BoxSizing::BorderBox,
            position: Position::Fixed,
            inset: Edges::new(
                Length::cells(0.0).into(),
                LengthAuto::Auto,
                LengthAuto::Auto,
                Length::cells(0.0).into()
            ),
            size: LayoutSize::new(
                Dimension::from(Length::vw(100.0)),
                Dimension::from(Length::vh(100.0))
            ),
            z_index: ZIndex::Integer(i32::MAX),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            bg: Color::Reset,
            fg: Color::White,
            overflow: Axes::all(Overflow::Hidden),

            &__header => {
                size: LayoutSize::new(
                    Dimension::Auto,
                    Dimension::from(Length::cells(1.0))
                ),
                flex_shrink: 0.0,
                overflow: Axes::all(Overflow::Clip),
                fg: Color::LightCyan,
                modifier: Modifier::BOLD
            }

            &__panes => {
                display: Display::Grid,
                grid_template_columns: vec![
                    fractional_track(0.75),
                    fractional_track(1.0),
                    fractional_track(1.0)
                ],
                grid_template_rows: vec![fractional_track(1.0)],
                flex_basis: Dimension::from(Length::cells(0.0)),
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_size: LayoutSize::new(
                    Dimension::from(Length::cells(0.0)),
                    Dimension::from(Length::cells(0.0))
                ),
                overflow: Axes::all(Overflow::Hidden)

                @media (max-width: 79) {
                    grid_template_columns: vec![
                        fractional_track(1.0),
                        fractional_track(1.0)
                    ]
                }

                @media (max-width: 49) {
                    grid_template_columns: vec![fractional_track(1.0)]
                }
            }

            &__parent => {
                borders: Borders::ALL,
                border_type: BorderType::Plain,
                fg: Color::DarkGray,
                overflow: Axes::all(Overflow::Hidden)

                @media (max-width: 79) { display: Display::None }
            }

            &__current => {
                borders: Borders::ALL,
                border_type: BorderType::Rounded,
                overflow: Axes::all(Overflow::Hidden)
            }

            &__preview => {
                borders: Borders::ALL,
                border_type: BorderType::Plain,
                overflow: Axes::all(Overflow::Hidden)

                @media (max-width: 49) { display: Display::None }
            }

            &__source => { borders: Borders::NONE }

            &__row => {
                borders: Borders::NONE,
                border_type: BorderType::Plain,
                padding: TuiSpacing::ZERO,
                fg: Color::White,
                bg: Color::Reset

                &:focus => {
                    fg: Color::Black,
                    bg: Color::LightCyan,
                    modifier: Modifier::BOLD
                }
            }

            &__directory => { fg: Color::LightBlue }
            &__symlink => { fg: Color::LightYellow }
            &__error => { fg: Color::LightRed }
            &__footer => {
                size: LayoutSize::new(
                    Dimension::Auto,
                    Dimension::from(Length::cells(1.0))
                ),
                flex_shrink: 0.0,
                overflow: Axes::all(Overflow::Clip),
                fg: Color::DarkGray
            }
        }
    }

    let initial_error = initial_directory.as_ref().err().cloned();
    let directory = initial_directory.ok().unwrap_or_else(|| PathBuf::from("."));
    let filesystem = use_file_system(volume_root(&directory)).ok();
    let current_directory = ArcRwSignal::new(directory.clone());
    let entries = ArcRwSignal::new(Vec::<FileEntry>::new());
    let selected = ArcRwSignal::new(None::<usize>);
    let selection_target = ArcRwSignal::new(None::<PathBuf>);
    let show_hidden = ArcRwSignal::new(options.show_hidden);
    let error = ArcRwSignal::new(initial_error);
    let read_directory = ArcRwSignal::new(
        filesystem
            .as_ref()
            .map(|filesystem| filesystem.read_dir(&directory)),
    );

    watch_directory_operation(
        read_directory.clone(),
        entries.clone(),
        selected.clone(),
        selection_target.clone(),
        error.clone(),
        options.clone(),
        show_hidden.clone(),
    );

    let preview_generation = Arc::new(AtomicU64::new(0));
    let preview_worker = Arc::new(tokio::sync::Mutex::new(()));
    let action_preview_worker = Arc::clone(&preview_worker);
    let preview = Action::new(move |request: &PreviewRequest| {
        let request = request.clone();
        let worker = Arc::clone(&action_preview_worker);
        async move {
            tokio::time::sleep(PREVIEW_DEBOUNCE).await;
            if !request.is_current() {
                return None;
            }
            let _worker = worker.lock_owned().await;
            if !request.is_current() {
                return None;
            }
            let contents = load_preview(request.entry.clone()).await;
            request.is_current().then_some(contents)
        }
    });
    watch_preview(
        entries.clone(),
        selected.clone(),
        show_hidden.clone(),
        options.clone(),
        preview_generation,
        preview.clone(),
    );
    let parent_preview = Action::new(|directory: &PathBuf| {
        let directory = directory.clone();
        async move { load_parent_preview(directory).await }
    });
    let parent_directory = current_directory.clone();
    let parent_action = parent_preview.clone();
    leptos::prelude::Effect::watch_sync(
        move || parent_directory.get(),
        move |directory, _, _| parent_action.dispatch(directory.clone()),
        true,
    );

    let key_entries = entries.clone();
    let key_selected = selected.clone();
    let key_selection_target = selection_target.clone();
    let key_hidden = show_hidden.clone();
    let key_options = options.clone();
    let key_current = current_directory.clone();
    let key_read = read_directory.clone();
    let key_filesystem = filesystem.clone();
    let key_set_file = set_file.clone();
    let key_screens = screens.clone();
    let chord = ArcRwSignal::new(false);
    use_key_event(KeyEventKind::Press, move |key| {
        if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL {
            key_screens.dismiss();
            return KeyControl::Handled;
        }
        if key.modifiers != KeyModifiers::NONE && key.modifiers != KeyModifiers::CONTROL {
            return KeyControl::Pass;
        }
        if key.code != KeyCode::Char('g') {
            let _ = chord.try_set(false);
        }

        let visible = visible_entries(&key_entries, &key_options, &key_hidden);
        let page = 10_usize;
        match key.code {
            KeyCode::Char('q') => {
                key_screens.dismiss();
                KeyControl::Exit
            }
            KeyCode::Esc => {
                key_screens.dismiss();
                KeyControl::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                move_selection(&key_selected, visible.len(), -1);
                KeyControl::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                move_selection(&key_selected, visible.len(), 1);
                KeyControl::Handled
            }
            KeyCode::PageUp => {
                move_selection_by(&key_selected, visible.len(), -(page as isize));
                KeyControl::Handled
            }
            KeyCode::Char('b') if key.modifiers == KeyModifiers::CONTROL => {
                move_selection_by(&key_selected, visible.len(), -(page as isize));
                KeyControl::Handled
            }
            KeyCode::PageDown => {
                move_selection_by(&key_selected, visible.len(), page as isize);
                KeyControl::Handled
            }
            KeyCode::Char('f') if key.modifiers == KeyModifiers::CONTROL => {
                move_selection_by(&key_selected, visible.len(), page as isize);
                KeyControl::Handled
            }
            KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
                move_selection_by(&key_selected, visible.len(), -((page / 2) as isize));
                KeyControl::Handled
            }
            KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
                move_selection_by(&key_selected, visible.len(), (page / 2) as isize);
                KeyControl::Handled
            }
            KeyCode::Home => {
                let _ = key_selected.try_set((!visible.is_empty()).then_some(0));
                KeyControl::Handled
            }
            KeyCode::End | KeyCode::Char('G') => {
                let _ = key_selected.try_set(visible.len().checked_sub(1));
                KeyControl::Handled
            }
            KeyCode::Char('g') => {
                if chord.get_untracked() {
                    let _ = chord.try_set(false);
                    let _ = key_selected.try_set((!visible.is_empty()).then_some(0));
                } else {
                    let _ = chord.try_set(true);
                }
                KeyControl::Handled
            }
            KeyCode::Char('.') => {
                let _ = key_hidden.try_update(|shown| *shown = !*shown);
                let next = visible_entries(&key_entries, &key_options, &key_hidden);
                let _ = key_selected.try_set((!next.is_empty()).then_some(0));
                KeyControl::Handled
            }
            KeyCode::Left | KeyCode::Char('h') => {
                let current = key_current.get_untracked();
                if let (Some(filesystem), Some(parent)) = (
                    key_filesystem.as_ref(),
                    current.parent().map(Path::to_path_buf),
                ) {
                    let _ = key_selection_target.try_set(Some(current));
                    browse_to(parent, filesystem, &key_current, &key_read, &key_selected);
                }
                KeyControl::Handled
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if let Some(index) = key_selected.get_untracked()
                    && let Some(entry) = visible.get(index).cloned()
                    && entry.kind() == FileKind::Directory
                {
                    activate_entry(
                        entry,
                        key_filesystem.as_ref(),
                        &key_current,
                        &key_read,
                        &key_selected,
                        &key_set_file,
                        &key_screens,
                    );
                }
                KeyControl::Handled
            }
            KeyCode::Enter => {
                if let Some(index) = key_selected.get_untracked()
                    && let Some(entry) = visible.get(index).cloned()
                {
                    activate_entry(
                        entry,
                        key_filesystem.as_ref(),
                        &key_current,
                        &key_read,
                        &key_selected,
                        &key_set_file,
                        &key_screens,
                    );
                }
                KeyControl::Handled
            }
            _ => KeyControl::Pass,
        }
    });

    let header_directory = current_directory.clone();
    let parent_value = parent_preview.value();
    let parent_pending = parent_preview.pending();
    let list_entries = entries.clone();
    let list_selected = selected.clone();
    let list_hidden = show_hidden.clone();
    let list_options = options.clone();
    let list_current = current_directory.clone();
    let list_read = read_directory.clone();
    let list_filesystem = filesystem.clone();
    let list_set_file = set_file.clone();
    let list_screens = screens.clone();
    let list_error = error.clone();
    let list_key_entries = entries.clone();
    let list_key_selected = selected.clone();
    let list_key_hidden = show_hidden.clone();
    let list_key_error = error.clone();
    let preview_value = preview.value();
    let preview_pending = preview.pending();
    let file_list = keyed(
        move || {
            (
                list_key_entries.get(),
                list_key_selected.get(),
                list_key_hidden.get(),
                list_key_error.get(),
            )
        },
        move || {
            let visible = visible_entries(&list_entries, &list_options, &list_hidden);
            let selected_index = list_selected.get_untracked();
            let mut rows = visible
                .into_iter()
                .enumerate()
                .map(|(index, entry)| {
                    let action_entry = entry.clone();
                    let action_filesystem = list_filesystem.clone();
                    let action_current = list_current.clone();
                    let action_read = list_read.clone();
                    let action_selected = list_selected.clone();
                    let action_set_file = list_set_file.clone();
                    let action_screens = list_screens.clone();
                    let classes = entry_classes(&entry);
                    let focused = selected_index == Some(index);
                    let row = button(entry_label(&entry))
                        .with_label_alignment(Alignment::Left)
                        .on_press(move || {
                            activate_entry(
                                action_entry.clone(),
                                action_filesystem.as_ref(),
                                &action_current,
                                &action_read,
                                &action_selected,
                                &action_set_file,
                                &action_screens,
                            );
                            AppControl::Continue
                        })
                        .with_classes(classes)
                        .with_focus(focused);
                    if focused {
                        row.metadata.request_scroll_into_view();
                    }
                    row.into_view()
                })
                .collect::<Vec<_>>();
            if rows.is_empty() {
                rows.push(text("No matching files").into_view());
            }
            if let Some(message) = list_error.get_untracked() {
                rows.push(
                    text(format!("Error: {message}"))
                        .with_classes("file-selector__error")
                        .into_view(),
                );
            }
            div(rows).into_view()
        },
    );

    view! {
        <Div class="file-selector">
            <Text class="file-selector__header">
                {move || format!("Select a file — {}", header_directory.get().display())}
            </Text>
            <Div class="file-selector__panes">
                <Block class="file-selector__parent">
                    {move || {
                        if parent_pending.get() {
                            return text("Loading parent…").into_view();
                        }
                        match parent_value.get() {
                            Some(ParentPreviewContent::Entries(entries)) => {
                                if entries.is_empty() {
                                    text("No parent entries").into_view()
                                } else {
                                    div(entries.into_iter().map(|entry| {
                                        let current = entry.current;
                                        let row = text(entry.label());
                                        if current {
                                            row.metadata.request_scroll_to_anchor();
                                        }
                                        row.into_view()
                                    }).collect::<Vec<_>>()).into_view()
                                }
                            }
                            Some(ParentPreviewContent::Text(contents)) => text(contents).into_view(),
                            None => text("No parent").into_view(),
                        }
                    }}
                </Block>
                <Block class="file-selector__current">
                    {file_list}
                </Block>
                <Block class="file-selector__preview">
                    {move || {
                        if preview_pending.get() {
                            return text("Loading preview…").into_view();
                        }
                        match preview_value.get().flatten() {
                            Some(PreviewContent::Directory(entries)) => {
                                if entries.is_empty() {
                                    text("Empty directory").into_view()
                                } else {
                                    div(entries.into_iter().map(|entry| {
                                        let row = text(entry.label());
                                        match entry_color_class(entry.kind, entry.symlink) {
                                            Some(class) => row.with_classes(class).into_view(),
                                            None => row.into_view(),
                                        }
                                    }).collect::<Vec<_>>()).into_view()
                                }
                            }
                            Some(PreviewContent::Image(path)) => image(path).into_view(),
                            Some(PreviewContent::Source(source)) => {
                                code_block_from_prepared(source)
                                    .line_numbers(false)
                                    .with_classes("file-selector__source")
                                    .into_view()
                            }
                            Some(PreviewContent::Text(contents)) => text(contents).into_view(),
                            None => text("No preview").into_view(),
                        }
                    }}
                </Block>
            </Div>
            <Text class="file-selector__footer">
                "j/k move | h/l browse | Enter select | . hidden | Esc cancel"
            </Text>
        </Div>
    }
}

/// Watches the active directory operation and publishes its latest listing.
fn watch_directory_operation(
    operation: ArcRwSignal<Option<FileOperation<Vec<FileEntry>>>>,
    entries: ArcRwSignal<Vec<FileEntry>>,
    selected: ArcRwSignal<Option<usize>>,
    selection_target: ArcRwSignal<Option<PathBuf>>,
    error: ArcRwSignal<Option<String>>,
    options: FileSelectorOptions,
    show_hidden: ArcRwSignal<bool>,
) {
    let result_operation = operation.clone();
    leptos::prelude::Effect::watch_sync(
        move || {
            operation
                .try_with(|operation| {
                    operation
                        .as_ref()
                        .and_then(|operation| operation.version().try_get())
                })
                .flatten()
                .unwrap_or_default()
        },
        move |version, _, _| {
            if *version == 0 {
                return;
            }
            let Some(Some(operation)) = result_operation.try_get_untracked() else {
                return;
            };
            operation.value().with_untracked(|result| match result {
                Some(Ok(next)) => {
                    let _ = entries.try_set(next.clone());
                    let visible = visible_entries(&entries, &options, &show_hidden);
                    let target = selection_target.get_untracked();
                    let next = target
                        .as_ref()
                        .and_then(|path| visible.iter().position(|entry| entry.path() == path))
                        .or_else(|| (!visible.is_empty()).then_some(0));
                    let _ = selection_target.try_set(None);
                    let _ = selected.try_set(next);
                    let _ = error.try_set(None);
                }
                Some(Err(source)) => {
                    let _ = selection_target.try_set(None);
                    let _ = error.try_set(Some(source.to_string()));
                }
                None => {}
            });
        },
        true,
    );
}

/// Watches selection changes and dispatches the latest preview request.
///
/// # Arguments
///
/// * `entries` — Entries in the current directory.
/// * `selected` — Index of the highlighted visible entry.
/// * `show_hidden` — Whether hidden entries participate in selection.
/// * `options` — File-filter configuration used to derive visible entries.
/// * `latest_generation` — Shared generation that invalidates superseded work.
/// * `preview` — Action receiving debounced, generation-scoped requests.
fn watch_preview(
    entries: ArcRwSignal<Vec<FileEntry>>,
    selected: ArcRwSignal<Option<usize>>,
    show_hidden: ArcRwSignal<bool>,
    options: FileSelectorOptions,
    latest_generation: Arc<AtomicU64>,
    preview: Action<PreviewRequest, Option<PreviewContent>>,
) {
    leptos::prelude::Effect::watch_sync(
        move || {
            let _ = entries.get();
            let _ = show_hidden.get();
            let visible = visible_entries(&entries, &options, &show_hidden);
            selected.get().and_then(|index| visible.get(index).cloned())
        },
        move |entry, _, _| {
            let generation = latest_generation
                .fetch_add(1, AtomicOrdering::AcqRel)
                .wrapping_add(1);
            if let Some(entry) = entry {
                preview.dispatch(PreviewRequest {
                    entry: entry.clone(),
                    generation,
                    latest_generation: Arc::clone(&latest_generation),
                });
            } else {
                preview.clear();
            }
        },
        true,
    );
}

/// Loads a read-only preview outside the terminal event thread.
async fn load_preview(entry: FileEntry) -> PreviewContent {
    let path = entry.path().to_path_buf();
    if entry.kind() == FileKind::Directory {
        return match tokio::task::spawn_blocking(move || preview_directory(&path)).await {
            Ok(Ok(entries)) => PreviewContent::Directory(entries),
            Ok(Err(error)) => PreviewContent::Text(error),
            Err(error) => PreviewContent::Text(format!("Preview failed: {error}")),
        };
    }
    if is_image_path(&path) {
        return PreviewContent::Image(path);
    }
    match tokio::task::spawn_blocking(move || preview_regular_file(&path)).await {
        Ok(contents) => contents,
        Err(error) => PreviewContent::Text(format!("Preview failed: {error}")),
    }
}

/// Loads structured parent-directory rows outside the terminal event thread.
///
/// # Arguments
///
/// * `directory` — Current directory whose siblings should be previewed.
///
/// # Returns
///
/// A [`ParentPreviewContent`] value containing sorted rows with a retained
/// current-directory marker, or plain fallback text.
async fn load_parent_preview(directory: PathBuf) -> ParentPreviewContent {
    tokio::task::spawn_blocking(move || {
        let Some(parent) = directory.parent() else {
            return ParentPreviewContent::Text(String::from("Filesystem root"));
        };
        let current_name = directory.file_name().map(OsStr::to_os_string);
        match fs::read_dir(parent) {
            Ok(entries) => {
                let mut entries = entries
                    .filter_map(Result::ok)
                    .map(DirectoryPreviewEntry::from_dir_entry)
                    .collect::<Vec<_>>();
                entries.sort_by(|left, right| {
                    compare_entry_metadata(left.kind, &left.name, right.kind, &right.name)
                });
                ParentPreviewContent::Entries(
                    entries
                        .into_iter()
                        .map(|entry| {
                            let current = current_name.as_deref() == Some(entry.name.as_os_str());
                            ParentPreviewEntry { entry, current }
                        })
                        .collect(),
                )
            }
            Err(error) => ParentPreviewContent::Text(format!("Unable to preview parent: {error}")),
        }
    })
    .await
    .unwrap_or_else(|error| ParentPreviewContent::Text(format!("Parent preview failed: {error}")))
}

/// Loads sorted entry metadata for a directory preview.
///
/// # Arguments
///
/// * `path` — Directory whose immediate entries should be previewed.
///
/// # Returns
///
/// A [`Result`] containing sorted preview entries or a user-facing read error.
fn preview_directory(path: &Path) -> Result<Vec<DirectoryPreviewEntry>, String> {
    match fs::read_dir(path) {
        Ok(entries) => {
            let mut entries = entries
                .filter_map(Result::ok)
                .map(DirectoryPreviewEntry::from_dir_entry)
                .collect::<Vec<_>>();
            entries.sort_by(|left, right| {
                compare_entry_metadata(left.kind, &left.name, right.kind, &right.name)
            });
            Ok(entries)
        }
        Err(error) => Err(format!("Unable to preview directory: {error}")),
    }
}

/// Formats a bounded text preview or portable metadata fallback.
///
/// # Arguments
///
/// * `path` — Regular file whose contents or metadata should be previewed.
///
/// # Returns
///
/// A [`PreviewContent`] value containing prepared highlighted source or plain
/// fallback text.
fn preview_regular_file(path: &Path) -> PreviewContent {
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) => return PreviewContent::Text(format!("Unable to preview file: {error}")),
    };
    let metadata = match file.metadata() {
        Ok(metadata) => metadata,
        Err(error) => return PreviewContent::Text(format!("Unable to read metadata: {error}")),
    };
    let mut bytes = Vec::with_capacity(MAX_TEXT_PREVIEW_BYTES as usize);
    if let Err(error) = file
        .by_ref()
        .take(MAX_TEXT_PREVIEW_BYTES)
        .read_to_end(&mut bytes)
    {
        return PreviewContent::Text(format!("Unable to preview file: {error}"));
    }
    let byte_truncated = metadata.len() > MAX_TEXT_PREVIEW_BYTES;
    let contents = match std::str::from_utf8(&bytes) {
        Ok(contents) => contents,
        Err(error) if byte_truncated && error.error_len().is_none() => {
            std::str::from_utf8(&bytes[..error.valid_up_to()])
                .expect("the valid UTF-8 prefix should decode")
        }
        Err(_) => {
            return PreviewContent::Text(format!(
                "{}\n{} bytes\nBinary file",
                path.display(),
                metadata.len()
            ));
        }
    };
    let mut lines = contents.split_inclusive('\n');
    let mut excerpt = lines
        .by_ref()
        .take(MAX_TEXT_PREVIEW_LINES)
        .collect::<String>();
    let truncated = byte_truncated || lines.next().is_some();
    if truncated {
        if !excerpt.is_empty() && !excerpt.ends_with('\n') {
            excerpt.push('\n');
        }
        excerpt.push_str("… Preview truncated …");
    }
    let language = preview_language(path);
    PreviewContent::Source(prepare_code_block(&excerpt, language.as_deref()))
}

/// Returns the syntax-language token inferred from a file extension.
///
/// # Arguments
///
/// * `path` — File path whose extension should select a bundled grammar.
///
/// # Returns
///
/// An optional lowercase [`String`] containing the extension token.
fn preview_language(path: &Path) -> Option<String> {
    path.extension()
        .and_then(OsStr::to_str)
        .map(str::to_ascii_lowercase)
        .filter(|extension| !extension.is_empty())
}

/// Returns entries visible under the current filter and hidden-file setting.
fn visible_entries(
    entries: &ArcRwSignal<Vec<FileEntry>>,
    options: &FileSelectorOptions,
    show_hidden: &ArcRwSignal<bool>,
) -> Vec<FileEntry> {
    let shown = show_hidden.get_untracked();
    let mut entries = entries
        .get_untracked()
        .into_iter()
        .filter(|entry| shown || !entry.name().to_string_lossy().starts_with('.'))
        .filter(|entry| {
            entry.kind() == FileKind::Directory
                || (entry.kind() == FileKind::File && options.permits(entry.path()))
        })
        .collect::<Vec<_>>();
    entries.sort_by(compare_entries);
    entries
}

/// Compares entries using directory-first case-insensitive ordering.
fn compare_entries(left: &FileEntry, right: &FileEntry) -> Ordering {
    compare_entry_metadata(left.kind(), left.name(), right.kind(), right.name())
}

/// Compares filesystem metadata using the selector's shared entry ordering.
///
/// # Arguments
///
/// * `left_kind` — Classification of the left entry.
/// * `left_name` — Visible name of the left entry.
/// * `right_kind` — Classification of the right entry.
/// * `right_name` — Visible name of the right entry.
///
/// # Returns
///
/// An [`Ordering`] that places directories first and compares names
/// case-insensitively with an exact-name tie-breaker.
fn compare_entry_metadata(
    left_kind: FileKind,
    left_name: &OsStr,
    right_kind: FileKind,
    right_name: &OsStr,
) -> Ordering {
    kind_rank(left_kind)
        .cmp(&kind_rank(right_kind))
        .then_with(|| {
            left_name
                .to_string_lossy()
                .to_ascii_lowercase()
                .cmp(&right_name.to_string_lossy().to_ascii_lowercase())
        })
        .then_with(|| left_name.cmp(right_name))
}

/// Returns the sorting rank for one entry kind.
const fn kind_rank(kind: FileKind) -> u8 {
    match kind {
        FileKind::Directory => 0,
        FileKind::File => 1,
        FileKind::Other => 2,
    }
}

/// Moves selection by one row in the requested direction.
fn move_selection(selected: &ArcRwSignal<Option<usize>>, len: usize, direction: isize) {
    move_selection_by(selected, len, direction);
}

/// Moves selection by a signed row count while clamping to the listing.
fn move_selection_by(selected: &ArcRwSignal<Option<usize>>, len: usize, amount: isize) {
    if len == 0 {
        let _ = selected.try_set(None);
        return;
    }
    let current = selected.get_untracked().unwrap_or_default();
    let next = current.saturating_add_signed(amount).min(len - 1);
    let _ = selected.try_set(Some(next));
}

/// Opens a directory entry or stores a regular-file selection.
fn activate_entry(
    entry: FileEntry,
    filesystem: Option<&FileSystem>,
    current: &ArcRwSignal<PathBuf>,
    operation: &ArcRwSignal<Option<FileOperation<Vec<FileEntry>>>>,
    selected: &ArcRwSignal<Option<usize>>,
    set_file: &ArcWriteSignal<Option<PathBuf>>,
    screens: &StandaloneScreenRegistry,
) {
    match entry.kind() {
        FileKind::Directory => {
            if let Some(filesystem) = filesystem {
                browse_to(
                    entry.path().to_path_buf(),
                    filesystem,
                    current,
                    operation,
                    selected,
                );
            }
        }
        FileKind::File => {
            let _ = set_file.try_set(Some(entry.path().to_path_buf()));
            screens.dismiss();
            request_redraw();
        }
        FileKind::Other => {}
    }
}

/// Dispatches a listing request for a new current directory.
fn browse_to(
    directory: PathBuf,
    filesystem: &FileSystem,
    current: &ArcRwSignal<PathBuf>,
    operation: &ArcRwSignal<Option<FileOperation<Vec<FileEntry>>>>,
    selected: &ArcRwSignal<Option<usize>>,
) {
    let _ = current.try_set(directory.clone());
    let _ = selected.try_set(None);
    let _ = operation.try_set(Some(filesystem.read_dir(directory)));
}

/// Returns the volume root containing a canonical path.
fn volume_root(path: &Path) -> PathBuf {
    path.ancestors().last().unwrap_or(path).to_path_buf()
}

/// Creates one fractionally sized explicit grid track.
///
/// # Arguments
///
/// * `fraction` — Fractional weight assigned to the track.
///
/// # Returns
///
/// A [`GridTemplateTrack`] containing the fractional weight.
fn fractional_track(fraction: f32) -> GridTemplateTrack {
    GridTemplateTrack::from(GridTrackSize::from(Fraction::new(fraction)))
}

/// Returns the display label for one selector entry.
fn entry_label(entry: &FileEntry) -> String {
    format_entry_label(entry.name(), entry.kind(), entry.is_symlink())
}

/// Formats one filesystem entry using the selector's shared icon convention.
///
/// # Arguments
///
/// * `name` — Visible filesystem entry name.
/// * `kind` — Classification selecting the entry icon.
/// * `symlink` — Whether to append the symbolic-link indicator.
///
/// # Returns
///
/// A [`String`] containing the icon, name, and optional symlink suffix.
fn format_entry_label(name: &OsStr, kind: FileKind, symlink: bool) -> String {
    let marker = match kind {
        FileKind::Directory => "󰉋",
        FileKind::File => "󰈔",
        FileKind::Other => "?",
    };
    let symlink = if symlink { " →" } else { "" };
    format!("{marker} {}{symlink}", name.to_string_lossy())
}

/// Returns the internal classes for one selector row.
fn entry_classes(entry: &FileEntry) -> &'static str {
    match entry_color_class(entry.kind(), entry.is_symlink()) {
        Some("file-selector__symlink") => "file-selector__row file-selector__symlink",
        Some("file-selector__directory") => "file-selector__row file-selector__directory",
        _ => "file-selector__row",
    }
}

/// Returns the selector color class for filesystem entry metadata.
///
/// Symbolic links take precedence over their followed target kind.
///
/// # Arguments
///
/// * `kind` — Classification of the followed filesystem target.
/// * `symlink` — Whether the visible entry is a symbolic link.
///
/// # Returns
///
/// An optional class name selecting the shared directory or symlink color.
fn entry_color_class(kind: FileKind, symlink: bool) -> Option<&'static str> {
    if symlink {
        Some("file-selector__symlink")
    } else if kind == FileKind::Directory {
        Some("file-selector__directory")
    } else {
        None
    }
}

/// Returns whether a path uses an image extension supported by the image view.
fn is_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "tif" | "tiff" | "ico"
            )
        })
}

#[cfg(test)]
/// Unit tests for file-selector configuration, storage, and previews.
mod tests {
    use std::{
        cell::RefCell,
        fs,
        rc::Rc,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    use crossterm::event::{Event, KeyEvent};
    use leptos::prelude::{Effect, Owner};
    use ratatui::{
        Terminal,
        backend::TestBackend,
        symbols::{block as symbol_block, line as symbol_line},
    };

    use crate::{
        RenderCtx, View, code_block, context::provide_context,
        view::core::layout::render_with_layout,
    };

    /// Creates a collision-resistant temporary test directory.
    ///
    /// # Arguments
    ///
    /// * `label` — Readable suffix included in the directory name.
    ///
    /// # Returns
    ///
    /// A [`PathBuf`] containing the created directory.
    fn temporary_directory(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should follow the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "leptatui-file-selector-{}-{label}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("temporary selector directory should be created");
        path
    }

    /// Renders the active selector after allowing asynchronous previews to settle.
    ///
    /// # Arguments
    ///
    /// * `screens` — Registry containing the active file-selector screen.
    /// * `width` — Terminal width used to resolve responsive styles.
    /// * `height` — Terminal height available to the selector.
    ///
    /// # Returns
    ///
    /// A [`Terminal`] containing the latest rendered selector frame.
    async fn render_active_selector(
        screens: &StandaloneScreenRegistry,
        width: u16,
        height: u16,
    ) -> Terminal<TestBackend> {
        tokio::task::spawn_blocking(|| {
            let _prepared = prepare_code_block("", Some("md"));
        })
        .await
        .expect("syntax preview services should initialize");
        let mut terminal =
            Terminal::new(TestBackend::new(width, height)).expect("test terminal should open");
        for _ in 0..16 {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            let screen = screens.active().expect("selector should remain mounted");
            terminal
                .draw(|frame| {
                    let mut context = RenderCtx::new(frame);
                    render_with_layout(&screen, &mut context, |context| {
                        View::render(&screen, context)
                    })
                    .expect("selector should render");
                })
                .expect("selector frame should draw");
        }
        terminal
    }

    /// Renders the active selector once without yielding to asynchronous preview work.
    ///
    /// # Arguments
    ///
    /// * `screens` — Registry containing the active file-selector screen.
    /// * `width` — Terminal width used to resolve responsive styles.
    /// * `height` — Terminal height available to the selector.
    ///
    /// # Returns
    ///
    /// A [`Terminal`] containing the immediate selector frame.
    fn render_active_selector_immediately(
        screens: &StandaloneScreenRegistry,
        width: u16,
        height: u16,
    ) -> Terminal<TestBackend> {
        let mut terminal =
            Terminal::new(TestBackend::new(width, height)).expect("test terminal should open");
        let screen = screens.active().expect("selector should remain mounted");
        terminal
            .draw(|frame| {
                let mut context = RenderCtx::new(frame);
                render_with_layout(&screen, &mut context, |context| {
                    View::render(&screen, context)
                })
                .expect("selector should render");
            })
            .expect("selector frame should draw");
        terminal
    }

    /// Returns every coordinate containing a requested terminal symbol.
    ///
    /// # Arguments
    ///
    /// * `terminal` — Rendered terminal buffer to inspect.
    /// * `symbol` — Exact cell symbol to locate.
    /// * `width` — Terminal width used to convert buffer indexes into coordinates.
    ///
    /// # Returns
    ///
    /// A [`Vec`] containing matching coordinates in buffer order.
    fn symbol_positions(
        terminal: &Terminal<TestBackend>,
        symbol: &str,
        width: u16,
    ) -> Vec<(u16, u16)> {
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .enumerate()
            .filter(|(_, cell)| cell.symbol() == symbol)
            .map(|(index, _)| {
                let index = index as u16;
                (index % width, index / width)
            })
            .collect()
    }

    /// Returns every symbol painted into a rendered terminal buffer.
    ///
    /// # Arguments
    ///
    /// * `terminal` — Rendered terminal buffer to inspect.
    ///
    /// # Returns
    ///
    /// A [`String`] containing terminal symbols in row-major order.
    fn rendered_text(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    /// Returns the symbols painted into one rendered terminal row.
    ///
    /// # Arguments
    ///
    /// * `terminal` — Rendered terminal buffer to inspect.
    /// * `width` — Terminal width used to locate the requested row.
    /// * `row` — Zero-based terminal row to inspect.
    ///
    /// # Returns
    ///
    /// A [`String`] containing the requested row's terminal symbols.
    fn rendered_row(terminal: &Terminal<TestBackend>, width: u16, row: u16) -> String {
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .skip(usize::from(width) * usize::from(row))
            .take(usize::from(width))
            .map(|cell| cell.symbol())
            .collect()
    }

    /// Returns symbols painted with the selector's focused-row background.
    ///
    /// # Arguments
    ///
    /// * `terminal` — Rendered terminal buffer to inspect.
    ///
    /// # Returns
    ///
    /// A [`String`] containing symbols from highlighted cells in buffer order.
    fn highlighted_text(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .filter(|cell| cell.bg == Color::LightCyan)
            .map(|cell| cell.symbol())
            .collect()
    }

    /// Verifies selector options normalize extensions and preserve defaults.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// extensions = [".MD", "markdown", "md"]
    /// show_hidden = default
    /// ```
    ///
    /// # Assertions
    ///
    /// - Hidden files are visible by default.
    /// - Duplicate extensions normalize to a sorted lowercase allowlist.
    /// - Markdown extensions match case-insensitively.
    /// - An unrelated extension is rejected.
    #[test]
    fn options_normalize_extension_filters() {
        let options = FileSelectorOptions::new().extensions([".MD", "markdown", "md"]);

        assert!(options.show_hidden);
        assert_eq!(options.extensions, ["markdown", "md"]);
        assert!(options.permits(Path::new("README.MD")));
        assert!(options.permits(Path::new("guide.markdown")));
        assert!(!options.permits(Path::new("notes.txt")));
    }

    /// Verifies explicit clearing removes the stored selector path.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// stored = /tmp/guide.md
    /// clear()
    /// ```
    ///
    /// # Assertions
    ///
    /// - `get_file()` returns the stored path before clearing.
    /// - `get_file()` returns `None` after clearing.
    #[test]
    fn clear_removes_the_stored_file() {
        let (file, set_file) = arc_signal(Some(PathBuf::from("/tmp/guide.md")));
        let selector = FileSelector {
            screens: None,
            file,
            set_file,
        };

        assert_eq!(selector.get_file(), Some(PathBuf::from("/tmp/guide.md")));
        selector.clear();
        assert_eq!(selector.get_file(), None);
    }

    /// Verifies `get_file` subscribes an effect to stored-file changes.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// Effect::new(|| selector.get_file())
    /// stored = /tmp/guide.md
    /// ```
    ///
    /// # Assertions
    ///
    /// - The effect initially observes no selected file.
    /// - Storing a path reruns the effect with that path.
    #[tokio::test(flavor = "current_thread")]
    async fn get_file_is_a_tracked_read() {
        tokio::task::LocalSet::new()
            .run_until(async {
                crate::executor::init_tokio_executor();
                let (file, set_file) = arc_signal(None::<PathBuf>);
                let selector = FileSelector {
                    screens: None,
                    file,
                    set_file,
                };
                let observed = Rc::new(RefCell::new(Vec::new()));
                let effect_selector = selector.clone();
                let effect_observed = Rc::clone(&observed);
                let owner = Owner::new();
                owner.with(|| {
                    Effect::new(move || {
                        effect_observed
                            .borrow_mut()
                            .push(effect_selector.get_file());
                    });
                });

                tokio::task::yield_now().await;
                selector.set_file.set(Some(PathBuf::from("/tmp/guide.md")));
                tokio::task::yield_now().await;

                assert_eq!(
                    observed.borrow().as_slice(),
                    [None, Some(PathBuf::from("/tmp/guide.md"))]
                );
            })
            .await;
    }

    /// Verifies regular-file previews retain only bounded text prefixes.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// guide.md = "# Guide"
    /// data.bin = [0xff, 0xfe]
    /// large.txt = 8 KiB + 1 byte
    /// lines.txt = 65 lines
    /// unicode.txt = UTF-8 character crossing the 8 KiB boundary
    /// ```
    ///
    /// # Assertions
    ///
    /// - UTF-8 contents are prepared with their lowercase extension grammar.
    /// - Invalid UTF-8 produces a binary metadata fallback.
    /// - Byte-limited files retain only their first 8 KiB and a truncation marker.
    /// - Line-limited files retain only their first 64 lines and a truncation marker.
    /// - A partial trailing UTF-8 character is removed without classifying the file as binary.
    #[test]
    fn regular_file_previews_are_bounded() {
        let root = temporary_directory("previews");
        let text_path = root.join("guide.md");
        let binary_path = root.join("data.bin");
        let large_path = root.join("large.txt");
        let lines_path = root.join("lines.txt");
        let unicode_path = root.join("unicode.txt");
        fs::write(&text_path, "# Guide").expect("text fixture should be written");
        fs::write(&binary_path, [0xff, 0xfe]).expect("binary fixture should be written");
        fs::write(&large_path, vec![b'a'; MAX_TEXT_PREVIEW_BYTES as usize + 1])
            .expect("large fixture should be written");
        let lines = (0..=MAX_TEXT_PREVIEW_LINES)
            .map(|index| format!("Line {index}\n"))
            .collect::<String>();
        fs::write(&lines_path, lines).expect("many-line fixture should be written");
        let mut unicode = vec![b'a'; MAX_TEXT_PREVIEW_BYTES as usize - 1];
        unicode.extend_from_slice("💡".as_bytes());
        fs::write(&unicode_path, unicode).expect("Unicode fixture should be written");

        assert_eq!(
            preview_regular_file(&text_path),
            PreviewContent::Source(prepare_code_block("# Guide", Some("md")))
        );
        assert!(matches!(
            preview_regular_file(&binary_path),
            PreviewContent::Text(message) if message.contains("Binary file")
        ));
        let byte_excerpt = format!(
            "{}\n… Preview truncated …",
            "a".repeat(MAX_TEXT_PREVIEW_BYTES as usize)
        );
        assert_eq!(
            preview_regular_file(&large_path),
            PreviewContent::Source(prepare_code_block(&byte_excerpt, Some("txt")))
        );
        let line_excerpt = format!(
            "{}… Preview truncated …",
            (0..MAX_TEXT_PREVIEW_LINES)
                .map(|index| format!("Line {index}\n"))
                .collect::<String>()
        );
        assert_eq!(
            preview_regular_file(&lines_path),
            PreviewContent::Source(prepare_code_block(&line_excerpt, Some("txt")))
        );
        let unicode_excerpt = format!(
            "{}\n… Preview truncated …",
            "a".repeat(MAX_TEXT_PREVIEW_BYTES as usize - 1)
        );
        assert_eq!(
            preview_regular_file(&unicode_path),
            PreviewContent::Source(prepare_code_block(&unicode_excerpt, Some("txt")))
        );

        fs::remove_dir_all(root).expect("temporary selector directory should be removed");
    }

    /// Verifies directory previews use shared entry icons and ordering.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// directory = [alpha.md, Beta/]
    /// other symlink label = socket
    /// ```
    ///
    /// # Assertions
    ///
    /// - The preview fixture file and directory are created successfully.
    /// - Preview entries retain the center column's directory-first ordering.
    /// - Names within one entry kind retain case-insensitive alphabetical ordering.
    /// - Regular files and directories use the selector's shared icons.
    /// - Other symbolic links retain the fallback icon and arrow suffix.
    /// - Directory and symbolic-link metadata select the shared color classes.
    #[test]
    fn directory_previews_include_shared_entry_icons() {
        let root = temporary_directory("directory-icons");
        fs::write(root.join("alpha.md"), "# Alpha").expect("Markdown fixture should be written");
        fs::create_dir(root.join("Beta")).expect("directory fixture should be created");

        let entries = preview_directory(&root).expect("directory preview should load");
        assert_eq!(
            entries
                .iter()
                .map(DirectoryPreviewEntry::label)
                .collect::<Vec<_>>(),
            ["󰉋 Beta", "󰈔 alpha.md"]
        );
        assert_eq!(entries[0].kind, FileKind::Directory);
        assert_eq!(entries[1].kind, FileKind::File);
        assert_eq!(
            format_entry_label(OsStr::new("socket"), FileKind::Other, true),
            "? socket →"
        );
        assert_eq!(
            entry_color_class(FileKind::Directory, false),
            Some("file-selector__directory")
        );
        assert_eq!(
            entry_color_class(FileKind::Directory, true),
            Some("file-selector__symlink")
        );
        assert_eq!(entry_color_class(FileKind::File, false), None);

        fs::remove_dir_all(root).expect("temporary selector directory should be removed");
    }

    /// Verifies parent previews add icons without replacing the current marker.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// parent = [alpha.md, z-current/]
    /// current directory = z-current/
    /// ```
    ///
    /// # Assertions
    ///
    /// - The parent preview fixture is created successfully.
    /// - The preview retains structured entries for viewport anchoring.
    /// - The current directory retains the leading `>` marker and directory icon.
    /// - The directory precedes the alphabetically earlier file like the center column.
    /// - The sibling file retains the unselected indentation and file icon.
    #[tokio::test(flavor = "current_thread")]
    async fn parent_previews_include_icons_and_current_marker() {
        let root = temporary_directory("parent-icons");
        let current = root.join("z-current");
        fs::create_dir(&current).expect("current directory fixture should be created");
        fs::write(root.join("alpha.md"), "# Alpha").expect("file fixture should be written");

        let ParentPreviewContent::Entries(entries) = load_parent_preview(current).await else {
            panic!("parent preview should retain structured entries");
        };
        assert_eq!(
            entries
                .iter()
                .map(ParentPreviewEntry::label)
                .collect::<Vec<_>>(),
            ["> 󰉋 z-current", "  󰈔 alpha.md"]
        );
        assert!(entries[0].current);
        assert!(!entries[1].current);

        fs::remove_dir_all(root).expect("temporary selector directory should be removed");
    }

    /// Verifies the right directory preview uses the middle-column entry palette.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// current = [a-preview/, z-sibling/]
    /// a-preview = [alpha.rs, Beta/]
    /// terminal = 88x12
    /// ```
    ///
    /// # Assertions
    ///
    /// - The selector and directory fixtures render successfully.
    /// - The middle-column sibling directory uses light blue.
    /// - The right-column regular file uses white.
    /// - The right-column directory uses the same light blue as the middle column.
    #[tokio::test(flavor = "current_thread")]
    async fn directory_preview_matches_middle_column_colors() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let root = temporary_directory("directory-colors");
                let previewed = root.join("a-preview");
                fs::create_dir(&previewed).expect("previewed directory should be created");
                fs::create_dir(root.join("z-sibling"))
                    .expect("sibling directory should be created");
                fs::write(previewed.join("alpha.rs"), "fn alpha() {}")
                    .expect("source fixture should be written");
                fs::create_dir(previewed.join("Beta")).expect("nested directory should be created");
                let screens = StandaloneScreenRegistry::new();
                let selector = crate::context::hooks::__with_context_scope(|| {
                    provide_context(screens.clone());
                    use_file_selector()
                });
                selector.select_with_options(FileSelectorOptions::new().starting_directory(&root));

                let rendered = render_active_selector(&screens, 88, 12).await;
                let buffer = rendered.backend().buffer();
                let current_top = symbol_positions(&rendered, "╭", 88)[0].1;
                let preview_top = symbol_positions(&rendered, "┌", 88)
                    .into_iter()
                    .find(|(x, _)| *x == 56)
                    .expect("preview border should render")
                    .1;
                let middle_directory = buffer[(25, current_top.saturating_add(2))].fg;
                let preview_directory = buffer[(57, preview_top.saturating_add(1))].fg;
                let preview_file = buffer[(57, preview_top.saturating_add(2))].fg;

                assert_eq!(middle_directory, Color::LightBlue);
                assert_eq!(preview_file, Color::White);
                assert_eq!(preview_directory, middle_directory);

                screens.dismiss();
                fs::remove_dir_all(root).expect("temporary selector directory should be removed");
            })
            .await;
    }

    /// Verifies source previews use extension-selected syntax highlighting.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// main.RS = "fn main() {}"
    /// terminal = 88x12
    /// ```
    ///
    /// # Assertions
    ///
    /// - The UTF-8 preview is prepared with the lowercase `rs` language grammar.
    /// - The selector renders only the three pane borders without a nested source border.
    /// - The source starts at the preview content edge without a line-number gutter.
    /// - The first source token uses the bundled Rust syntax color.
    #[tokio::test(flavor = "current_thread")]
    async fn source_preview_is_borderless_and_syntax_highlighted() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let root = temporary_directory("source-highlighting");
                let source_path = root.join("main.RS");
                let contents = "fn main() {}";
                fs::write(&source_path, contents).expect("Rust fixture should be written");
                assert_eq!(
                    preview_regular_file(&source_path),
                    PreviewContent::Source(prepare_code_block(contents, Some("rs")))
                );
                let expected_color = code_block(contents).language("rs").highlighted_lines()[0]
                    .spans[0]
                    .style
                    .fg
                    .expect("Rust keyword should have a syntax color");
                let screens = StandaloneScreenRegistry::new();
                let selector = crate::context::hooks::__with_context_scope(|| {
                    provide_context(screens.clone());
                    use_file_selector()
                });
                selector.select_with_options(FileSelectorOptions::new().starting_directory(&root));

                let rendered = render_active_selector(&screens, 88, 12).await;
                let plain_borders = symbol_positions(&rendered, "┌", 88);
                let preview_top = plain_borders
                    .iter()
                    .find(|(x, _)| *x == 56)
                    .expect("preview border should render")
                    .1;
                let first_source =
                    &rendered.backend().buffer()[(57, preview_top.saturating_add(1))];

                assert_eq!(plain_borders.len(), 2);
                assert_eq!(first_source.symbol(), "f");
                assert_eq!(first_source.fg, expected_color);

                screens.dismiss();
                fs::remove_dir_all(root).expect("temporary selector directory should be removed");
            })
            .await;
    }

    /// Verifies tall listings and source previews remain clipped inside fixed selector chrome.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// files = 00-main.rs plus 01.txt through 19.txt
    /// 00-main.rs = forty source lines
    /// terminal = 88x8
    /// ```
    ///
    /// # Assertions
    ///
    /// - The header remains on the first terminal row.
    /// - The footer remains on the final terminal row.
    /// - All three pane borders retain the bounded area between the chrome rows.
    /// - Neither scrollbar thumbs nor scrollbar tracks are rendered.
    /// - Only source lines that fit in the preview pane are painted.
    #[tokio::test(flavor = "current_thread")]
    async fn tall_content_preserves_chrome_and_hides_scrollbars() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let root = temporary_directory("tall-content");
                let source = (0..40)
                    .map(|index| format!("fn line_{index:02}() {{}}\n"))
                    .collect::<String>();
                fs::write(root.join("00-main.rs"), source)
                    .expect("source fixture should be written");
                for index in 1..20 {
                    fs::write(
                        root.join(format!("{index:02}.txt")),
                        format!("File {index:02}"),
                    )
                    .expect("listing fixture should be written");
                }
                let screens = StandaloneScreenRegistry::new();
                let selector = crate::context::hooks::__with_context_scope(|| {
                    provide_context(screens.clone());
                    use_file_selector()
                });
                selector.select_with_options(FileSelectorOptions::new().starting_directory(&root));

                let rendered = render_active_selector(&screens, 88, 8).await;
                let contents = rendered_text(&rendered);

                assert!(rendered_row(&rendered, 88, 0).contains("Select a file"));
                assert!(rendered_row(&rendered, 88, 7).contains("j/k move"));
                assert_eq!(rendered.backend().buffer()[(0, 6)].symbol(), "└");
                assert_eq!(rendered.backend().buffer()[(24, 6)].symbol(), "╰");
                assert_eq!(rendered.backend().buffer()[(56, 6)].symbol(), "└");
                assert_eq!(rendered.backend().buffer()[(87, 6)].symbol(), "┘");
                assert!(symbol_positions(&rendered, symbol_block::FULL, 88).is_empty());
                assert!(symbol_positions(&rendered, symbol_line::DOUBLE_VERTICAL, 88).is_empty());
                assert!(contents.contains("line_00"));
                assert!(!contents.contains("line_39"));

                screens.dismiss();
                fs::remove_dir_all(root).expect("temporary selector directory should be removed");
            })
            .await;
    }

    /// Verifies the parent pane scrolls its clipped viewport to the current directory.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// parent = twenty sibling directories
    /// current directories = 10-current, then 19-current
    /// terminal = 88x8
    /// ```
    ///
    /// # Assertions
    ///
    /// - A current directory in the middle of the listing remains visible with its `>` marker.
    /// - A current directory at the end of the listing remains visible with its `>` marker.
    /// - The clipped parent pane does not render a scrollbar.
    #[tokio::test(flavor = "current_thread")]
    async fn parent_preview_scrolls_to_the_current_directory() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let root = temporary_directory("parent-anchor");
                let parent = root.join("parent");
                fs::create_dir(&parent).expect("parent fixture should be created");
                for index in 0..20 {
                    let name = if index == 10 || index == 19 {
                        format!("{index:02}-current")
                    } else {
                        format!("{index:02}-sibling")
                    };
                    fs::create_dir(parent.join(name))
                        .expect("sibling directory fixture should be created");
                }
                let screens = StandaloneScreenRegistry::new();
                let selector = crate::context::hooks::__with_context_scope(|| {
                    provide_context(screens.clone());
                    use_file_selector()
                });

                for index in [10, 19] {
                    selector.select_with_options(
                        FileSelectorOptions::new()
                            .starting_directory(parent.join(format!("{index:02}-current"))),
                    );
                    let rendered = render_active_selector(&screens, 88, 8).await;
                    assert!(rendered_text(&rendered).contains(&format!("> 󰉋 {index:02}-current")));
                    assert!(symbol_positions(&rendered, symbol_block::FULL, 88).is_empty());
                    assert!(
                        symbol_positions(&rendered, symbol_line::DOUBLE_VERTICAL, 88).is_empty()
                    );
                    screens.dismiss();
                }

                fs::remove_dir_all(root).expect("temporary selector directory should be removed");
            })
            .await;
    }

    /// Verifies selector panes retain their requested responsive proportions and alignment.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// widths = [88, 60, 40]
    /// panes = [parent, current, preview]
    /// ```
    ///
    /// # Assertions
    ///
    /// - A wide selector renders parent, current, and preview panes at 24, 32, and 32 cells.
    /// - A medium selector hides the parent and renders two equal thirty-cell panes.
    /// - A narrow selector hides both side panes and fills the width with the current pane.
    /// - Current-directory rows and preview text begin at their panes' left content edges.
    #[tokio::test(flavor = "current_thread")]
    async fn panes_are_weighted_responsive_and_left_aligned() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let root = temporary_directory("responsive-panes");
                fs::write(root.join("guide.md"), "# Guide")
                    .expect("Markdown fixture should be written");
                let screens = StandaloneScreenRegistry::new();
                let selector = crate::context::hooks::__with_context_scope(|| {
                    provide_context(screens.clone());
                    use_file_selector()
                });
                selector.select_with_options(FileSelectorOptions::new().starting_directory(&root));

                let wide = render_active_selector(&screens, 88, 12).await;
                let wide_buffer = wide.backend().buffer();
                let wide_plain = symbol_positions(&wide, "┌", 88);
                let wide_current = symbol_positions(&wide, "╭", 88);
                assert_eq!(wide_plain.len(), 2);
                assert_eq!([wide_plain[0].0, wide_plain[1].0], [0, 56]);
                assert_eq!(wide_current.len(), 1);
                assert_eq!(wide_current[0].0, 24);
                assert_eq!(wide_plain[0].1, wide_current[0].1);
                assert_eq!(wide_plain[1].1, wide_current[0].1);
                assert_ne!(
                    wide_buffer[(25, wide_current[0].1.saturating_add(1))].symbol(),
                    " "
                );
                assert_eq!(
                    wide_buffer[(57, wide_plain[1].1.saturating_add(1))].symbol(),
                    "#"
                );

                let medium = render_active_selector(&screens, 60, 12).await;
                let medium_buffer = medium.backend().buffer();
                let medium_plain = symbol_positions(&medium, "┌", 60);
                let medium_current = symbol_positions(&medium, "╭", 60);
                assert_eq!(medium_plain.len(), 1);
                assert_eq!(medium_plain[0].0, 30);
                assert_eq!(medium_current.len(), 1);
                assert_eq!(medium_current[0].0, 0);
                assert_eq!(medium_plain[0].1, medium_current[0].1);
                assert_ne!(
                    medium_buffer[(1, medium_current[0].1.saturating_add(1))].symbol(),
                    " "
                );
                assert_eq!(
                    medium_buffer[(31, medium_plain[0].1.saturating_add(1))].symbol(),
                    "#"
                );

                let narrow = render_active_selector(&screens, 40, 12).await;
                let narrow_buffer = narrow.backend().buffer();
                let narrow_plain = symbol_positions(&narrow, "┌", 40);
                let narrow_current = symbol_positions(&narrow, "╭", 40);
                assert!(narrow_plain.is_empty());
                assert_eq!(narrow_current.len(), 1);
                assert_eq!(narrow_current[0].0, 0);
                assert_ne!(
                    narrow_buffer[(1, narrow_current[0].1.saturating_add(1))].symbol(),
                    " "
                );

                screens.dismiss();
                fs::remove_dir_all(root).expect("temporary selector directory should be removed");
            })
            .await;
    }

    /// Verifies keyboard selection stays responsive while latest-only previews settle.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// files = 00.md through 19.md
    /// keys = Down, End before preview debounce completes
    /// terminal = 60x8
    /// ```
    ///
    /// # Assertions
    ///
    /// - The selector mounts and renders the generated listing.
    /// - Down immediately highlights `01.md` instead of retaining `00.md`.
    /// - End scrolls `19.md` into view and highlights it.
    /// - The settled preview contains `File 19` and no stale `File 01` content.
    #[tokio::test(flavor = "current_thread")]
    async fn keyboard_selection_updates_highlight_and_scrolls() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let root = temporary_directory("keyboard-highlight");
                for index in 0..20 {
                    fs::write(
                        root.join(format!("{index:02}.md")),
                        format!("File {index:02}"),
                    )
                    .expect("Markdown fixture should be written");
                }
                let screens = StandaloneScreenRegistry::new();
                let selector = crate::context::hooks::__with_context_scope(|| {
                    provide_context(screens.clone());
                    use_file_selector()
                });
                selector.select_with_options(FileSelectorOptions::new().starting_directory(&root));

                let _initial = render_active_selector(&screens, 60, 8).await;
                let mut screen = screens.active().expect("selector should remain mounted");
                View::handle_event(
                    &mut screen,
                    Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
                )
                .expect("Down should be handled");

                let moved = render_active_selector_immediately(&screens, 60, 8);
                let moved_highlight = highlighted_text(&moved);
                assert!(moved_highlight.contains("01.md"));
                assert!(!moved_highlight.contains("00.md"));

                let mut screen = screens.active().expect("selector should remain mounted");
                View::handle_event(
                    &mut screen,
                    Event::Key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE)),
                )
                .expect("End should be handled");

                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                let ended = render_active_selector(&screens, 60, 8).await;
                assert!(highlighted_text(&ended).contains("19.md"));
                let ended_contents = rendered_text(&ended);
                assert!(
                    ended_contents.contains("File 19"),
                    "latest preview should render: {ended_contents}"
                );
                assert!(!ended_contents.contains("File 01"));

                screens.dismiss();
                fs::remove_dir_all(root).expect("temporary selector directory should be removed");
            })
            .await;
    }

    /// Verifies parent navigation restores selection to the directory just exited.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// directory = [a-first/, z-child/]
    /// keys = End, Right, h
    /// ```
    ///
    /// # Assertions
    ///
    /// - The parent and child directory fixtures are created successfully.
    /// - Right enters the selected `z-child` directory.
    /// - `h` returns to the parent and highlights `z-child` instead of `a-first`.
    #[tokio::test(flavor = "current_thread")]
    async fn parent_navigation_selects_the_directory_just_exited() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let root = temporary_directory("parent-selection");
                fs::create_dir(root.join("a-first"))
                    .expect("first directory fixture should be created");
                fs::create_dir(root.join("z-child"))
                    .expect("child directory fixture should be created");
                let screens = StandaloneScreenRegistry::new();
                let selector = crate::context::hooks::__with_context_scope(|| {
                    provide_context(screens.clone());
                    use_file_selector()
                });
                selector.select_with_options(FileSelectorOptions::new().starting_directory(&root));

                let _initial = render_active_selector(&screens, 60, 8).await;
                let mut screen = screens.active().expect("selector should remain mounted");
                View::handle_event(
                    &mut screen,
                    Event::Key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE)),
                )
                .expect("End should be handled");
                View::handle_event(
                    &mut screen,
                    Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
                )
                .expect("Right should be handled");

                let _child = render_active_selector(&screens, 60, 8).await;
                let mut screen = screens.active().expect("selector should remain mounted");
                View::handle_event(
                    &mut screen,
                    Event::Key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE)),
                )
                .expect("h should be handled");

                let parent = render_active_selector(&screens, 60, 8).await;
                let highlight = highlighted_text(&parent);
                assert!(highlight.contains("z-child"));
                assert!(!highlight.contains("a-first"));

                screens.dismiss();
                fs::remove_dir_all(root).expect("temporary selector directory should be removed");
            })
            .await;
    }

    /// Verifies selector Quit exits the app while Escape only cancels the selector.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// selector + q
    /// selector + Esc
    /// ```
    ///
    /// # Assertions
    ///
    /// - Pressing `q` returns [`AppControl::Exit`].
    /// - Quit dismisses the owned selector before application teardown.
    /// - Pressing Escape returns [`AppControl::Continue`].
    /// - Escape dismisses the standalone selector screen.
    #[tokio::test(flavor = "current_thread")]
    async fn quit_exits_while_escape_cancels() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let root = temporary_directory("quit-and-cancel");
                let screens = StandaloneScreenRegistry::new();
                let selector = crate::context::hooks::__with_context_scope(|| {
                    provide_context(screens.clone());
                    use_file_selector()
                });

                selector.select_with_options(FileSelectorOptions::new().starting_directory(&root));
                let _quit_selector = render_active_selector(&screens, 60, 8).await;
                let mut screen = screens.active().expect("selector should remain mounted");
                assert_eq!(
                    View::handle_event(
                        &mut screen,
                        Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
                    )
                    .expect("q should be handled"),
                    AppControl::Exit,
                );
                assert!(screens.active().is_none());

                selector.select_with_options(FileSelectorOptions::new().starting_directory(&root));
                let _cancel_selector = render_active_selector(&screens, 60, 8).await;
                let mut screen = screens.active().expect("selector should remain mounted");
                assert_eq!(
                    View::handle_event(
                        &mut screen,
                        Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
                    )
                    .expect("Escape should be handled"),
                    AppControl::Continue,
                );
                assert!(screens.active().is_none());

                fs::remove_dir_all(root).expect("temporary selector directory should be removed");
            })
            .await;
    }

    /// Verifies Enter stores the highlighted file and dismisses the selector.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// directory = [guide.md]
    /// select_with_options(directory, extensions = [md])
    /// Enter
    /// ```
    ///
    /// # Assertions
    ///
    /// - The standalone selector mounts through the runner registry.
    /// - Enter stores the canonical highlighted file.
    /// - Successful selection dismisses the standalone screen.
    #[tokio::test(flavor = "current_thread")]
    async fn enter_stores_the_highlighted_file() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let root = temporary_directory("enter-selection");
                let guide = root.join("guide.md");
                fs::write(&guide, "# Guide").expect("Markdown fixture should be written");
                let canonical_guide =
                    fs::canonicalize(&guide).expect("Markdown fixture should canonicalize");
                let screens = StandaloneScreenRegistry::new();
                let selector = crate::context::hooks::__with_context_scope(|| {
                    provide_context(screens.clone());
                    use_file_selector()
                });

                selector.select_with_options(
                    FileSelectorOptions::new()
                        .starting_directory(&root)
                        .extensions(["md"]),
                );
                assert!(screens.active().is_some());

                let mut terminal =
                    Terminal::new(TestBackend::new(100, 24)).expect("test terminal should open");
                for _ in 0..4 {
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                    let screen = screens.active().expect("selector should remain mounted");
                    terminal
                        .draw(|frame| {
                            let mut context = RenderCtx::new(frame);
                            render_with_layout(&screen, &mut context, |context| {
                                View::render(&screen, context)
                            })
                            .expect("selector should render");
                        })
                        .expect("selector frame should draw");
                }

                let mut screen = screens.active().expect("selector should remain mounted");
                View::handle_event(
                    &mut screen,
                    Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
                )
                .expect("Enter should be handled");

                assert_eq!(selector.get_file(), Some(canonical_guide));
                assert!(screens.active().is_none());
                fs::remove_dir_all(root).expect("temporary selector directory should be removed");
            })
            .await;
    }
}
